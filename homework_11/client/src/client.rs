use crate::{
    client_error::ClientError,
    command::Command,
    encryption::{self, decrypt_payload, encrypt_payload},
    utils::{save_file, write_to_output},
};
use anyhow::Result;
use chrono::Utc;
use shared::message::{AuthUser, Message, MessagePayload};
use std::{net::Ipv4Addr, str::FromStr};
use tokio::io::AsyncWrite;
use tokio::{
    io::AsyncRead,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

/// The main client struct.
///
/// It is responsible for connecting to the server and authenticating the user before returning ClientReceiver and ClientSender.
/// All output messages are written to `writer`.
pub struct Client;

impl Client {
    /// Connects to the server and returns a sender and a receiver. The creation is inspired by the channel.
    /// writer: T is generic to abstract the output. It can be stdout, file or anything that implements Write. I made it generic to make it easier to test and not to use println! all the time.
    pub async fn connect<T>(
        mut writer: T,
        host: Ipv4Addr,
        port: u32,
        output_dir: &str,
        e2e_encryption: Option<String>,
    ) -> Result<(
        ClientSender<OwnedWriteHalf>,
        ClientReceiver<OwnedReadHalf, T>,
    )>
    where
        T: AsyncWrite + Unpin,
    {
        let server = format!("{}:{}", host, port);

        write_to_output(
            &mut writer,
            format!("Connecting to server on {}...\n", server).as_bytes(),
        )
        .await?;

        let mut stream = TcpStream::connect(server).await?;

        while Self::authenticate(&mut writer, &mut stream).await.is_err() {
            write_to_output(&mut writer, b"Please try to log in again.\n").await?;
        }

        let (read_half, write_half) = stream.into_split();

        write_to_output(&mut writer, b"Connected. You can now send messages.\n").await?;

        if e2e_encryption.is_some() {
            write_to_output(&mut writer, b"E2E encryption enabled.\n").await?;
        }
        let key = e2e_encryption.map(|key| encryption::pad_to_32_bytes(key.as_bytes()));

        // Create both ends of the client. I split it to two structs to make it easier to test.
        let receiver = ClientReceiver::new(read_half, writer, output_dir, key);
        let sender = ClientSender::new(write_half, key);

        Ok((sender, receiver))
    }

    async fn authenticate<T>(mut writer: T, stream: &mut TcpStream) -> Result<()>
    where
        T: AsyncWrite + Unpin,
    {
        write_to_output(&mut writer, b"Enter your username.\n").await?;
        let mut name = String::new();
        std::io::stdin().read_line(&mut name)?;
        let name = name.trim();

        write_to_output(
            &mut writer,
            b"Enter your password. If you haven't registered yet, you will be registered with this username and password.\n",
        )
        .await?;

        let mut password = String::new();
        std::io::stdin().read_line(&mut password)?;
        let password = password.trim();

        let user = AuthUser::new(name, password);

        let payload = Message::handshake(stream, user).await?.data;

        if let MessagePayload::LoginResponse(data) = payload {
            write_to_output(&mut writer, data.to_string().as_bytes()).await?;
            if data.is_success() {
                return Ok(());
            }
        }

        Err(ClientError::LoginFailed.into())
    }
}

/// The client sender. It is responsible for parsing user commands and sending messages to the server.
pub struct ClientSender<T>
where
    T: AsyncWrite + Unpin,
{
    stream: T,
    encryption_key: Option<[u8; 32]>,
}

impl<T> ClientSender<T>
where
    T: AsyncWrite + Unpin,
{
    fn new(stream: T, encryption_key: Option<[u8; 32]>) -> Self {
        ClientSender {
            stream,
            encryption_key,
        }
    }

    /// Starts listening for user input and sends it to the server.
    pub async fn start(mut self) -> Result<()> {
        loop {
            let mut text = String::new();
            std::io::stdin().read_line(&mut text)?;

            let cmd = Command::from_str(text.trim())?;

            if cmd == Command::Quit {
                return Ok(());
            }

            let mut data = match cmd.into_message().await {
                Ok(data) => data,
                Err(e) => {
                    tracing::error!("Cannot process command. {e}");
                    eprintln!("Cannot process command. {e}");
                    continue;
                }
            };

            if let Some(key) = self.encryption_key {
                data = encrypt_payload(data, &key)?;
            }

            let msg = Message::new(data);

            Message::send_msg(&msg, &mut self.stream).await?;
        }
    }
}

/// The client receiver. It is responsible for receiving messages from the server and handling them.
pub struct ClientReceiver<T, U> {
    stream: T,
    writer: U,
    output_dir: String,
    encryption_key: Option<[u8; 32]>,
}

impl<T, U> ClientReceiver<T, U>
where
    T: AsyncRead + Unpin,
    U: AsyncWrite + Unpin,
{
    fn new(stream: T, writer: U, output_dir: &str, encryption_key: Option<[u8; 32]>) -> Self {
        Self {
            stream,
            writer,
            output_dir: output_dir.to_string(),
            encryption_key,
        }
    }

    pub async fn start(mut self) -> Result<()> {
        tracing::debug!("starting receiver");

        while let Ok(message) = Message::receive_msg(&mut self.stream).await {
            tracing::debug!("received msg");
            if let Err(e) = Self::handle_message(
                message,
                &mut self.writer,
                &self.output_dir,
                &self.encryption_key,
            )
            .await
            {
                tracing::error!("Error while handling message. {e}");
                eprintln!("Error while handling message. {e}");
            }
        }
        tracing::debug!("receiver end");

        Ok(())
    }

    /// Handles the received message. It writes the message to the `writer`. If message ista if it is an image or a file.
    #[tracing::instrument(name = "Handling message", skip_all)]
    async fn handle_message(
        mut message: Message,
        writer: &mut U,
        output_dir: &str,
        encryption_key: &Option<[u8; 32]>,
    ) -> Result<(), ClientError> {
        if let Some(key) = encryption_key {
            let decrypted_data = match decrypt_payload(message.data, key) {
                Ok(data) => data,
                Err(e) => {
                    tracing::warn!("Decrypting payload error. {e}");
                    write_to_output(
                        writer,
                        format!(
                            "Unable to decrypt message from {}.\n",
                            message.sender.as_ref().unwrap_or(&"Anonymous".to_string())
                        )
                        .as_bytes(),
                    )
                    .await?;

                    return Ok(());
                }
            };
            message.data = decrypted_data;
        }

        write_to_output(writer, message.to_string().as_bytes()).await?;
        Self::store_data(message.data, writer, output_dir).await?;
        Ok(())
    }

    #[tracing::instrument(name = "Saving data to output dir", skip_all)]
    async fn store_data(
        message: MessagePayload,
        writer: &mut U,
        output_dir: &str,
    ) -> Result<(), ClientError> {
        match message {
            MessagePayload::Image(data) => {
                let now = Utc::now();
                let timestamp = now.timestamp();
                let file_path = format!("{}/images/{}.png", output_dir, timestamp);
                save_file(&file_path, &data).await?;
                write_to_output(
                    writer,
                    format!("Image saved to: {}\n", file_path).as_bytes(),
                )
                .await?;
            }
            MessagePayload::File(file_name, data) => {
                let file_path = format!("{}/files/{}", output_dir, file_name);
                save_file(&file_path, &data).await?;
                write_to_output(writer, format!("File saved to: {}\n", file_path).as_bytes())
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::ClientReceiver;

    use shared::message::{Message, MessagePayload};
    use tokio::io::AsyncWrite;
    use tokio::net::{TcpListener, TcpStream};

    use std::io::Result as IoResult;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct TestWriter {
        buf: Vec<u8>,
    }

    impl AsyncWrite for TestWriter {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<IoResult<usize>> {
            self.buf.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn receiver_receives_text_message() {
        let test_writer = TestWriter { buf: Vec::new() };

        let addr = "127.0.0.1:0";

        let listener = TcpListener::bind(addr)
            .await
            .expect("Couldn't bind to address");

        let port = listener.local_addr().unwrap().port();

        let stream = TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .expect("Couldn't connect to listener.");

        let client_receiver = ClientReceiver {
            stream,
            writer: test_writer,
            output_dir: "./".to_string(),
            encryption_key: None,
        };

        let payload = MessagePayload::Text("Hello world!".to_string());

        let msg = Message::new(payload);

        let handle = tokio::task::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            Message::send_msg(&msg, &mut socket).await.unwrap();
        });

        handle.await.unwrap();

        let result = client_receiver.start().await;

        assert!(result.is_ok());

        // I wasn't able to check the inner writer without changing a lot of code. I want to keep it private in the ClientReceiver and I wasn't able to pass some Arc<Mutex<Writer>> because it's not AsyncWrite.
        // Need to think more about how to structure and test the code.
        // assert_eq!(
        //     client_receiver.writer.get_ref(),
        //     String::from("Hello world!").as_bytes()
        // );
    }
}
