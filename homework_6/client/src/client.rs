use std::{
    error::Error,
    io::Write,
    net::{Ipv4Addr, TcpStream},
    str::FromStr,
};

use chrono::Utc;
use shared::message::{Message, MessagePayload};

use crate::{
    command::Command,
    utils::{log_error, save_file, write_to_output},
};
/// The main client struct.
pub struct Client;

impl Client {
    /// Connects to the server and returns a sender and a receiver. The creation is inspired by the channel.
    /// writer: T is generic to abstract the output. It can be stdout, file or anything that implements Write. I made it generic to make it easier to test and not to use stdout in tests.
    pub fn connect<T>(
        mut writer: T,
        host: Ipv4Addr,
        port: u32,
        output_dir: &str,
        username: &str,
    ) -> Result<(ClientSender, ClientReceiver<T>), Box<dyn Error>>
    where
        T: Write,
    {
        let server = format!("{}:{}", host, port);

        write_to_output(
            &mut writer,
            format!("Connecting to server on {}...\n", server).as_bytes(),
        )?;

        let mut stream = TcpStream::connect(server)?;
        stream.set_nodelay(true)?;

        // Once the connection is established, send the username to the server. It is then broadcasted for all clients.
        Message::send_new_user_msg(&mut stream, username)?;

        write_to_output(&mut writer, b"Connected. You can now send messages.\n")?;
        let receiver_stream = stream.try_clone()?;

        // Create both ends of the client. I split it to two structs to make it easier to test.
        let receiver = ClientReceiver::new(receiver_stream, writer, output_dir);
        let sender = ClientSender::new(stream, username.to_owned());

        Ok((sender, receiver))
    }
}

/// The client sender. It is responsible for parsing user commands and sending messages to the server.
pub struct ClientSender {
    stream: TcpStream,
    username: String,
}

impl ClientSender {
    fn new(stream: TcpStream, username: String) -> Self {
        ClientSender { stream, username }
    }

    /// Starts listening for user input and sends it to the server.
    pub fn start(mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let mut text = String::new();

            std::io::stdin().read_line(&mut text)?;

            let cmd = Command::from_str(text.trim())?;

            if cmd == Command::Quit {
                return Ok(());
            }

            let data = match cmd.try_into() {
                Ok(data) => data,
                Err(e) => {
                    log_error(e);
                    continue;
                }
            };

            let msg = Message::new(&self.username, data);

            Message::send_msg(&msg, &mut self.stream)?;
        }
    }
}

/// The client receiver. It is responsible for receiving messages from the server and handling them.
pub struct ClientReceiver<T> {
    writer: T,
    output_dir: String,
    stream: TcpStream,
}

impl<T> ClientReceiver<T>
where
    T: Write,
{
    fn new(stream: TcpStream, writer: T, output_dir: &str) -> Self {
        Self {
            stream,
            writer,
            output_dir: output_dir.to_string(),
        }
    }

    pub fn start(mut self) {
        while let Ok(message) = Message::receive_msg(&mut self.stream) {
            if let Err(e) = Self::handle_message(message, &mut self.writer, &self.output_dir) {
                log_error(e)
            }
        }
    }

    /// Handles the received message. It writes it to the writer and stores the data if it is an image or a file.
    #[tracing::instrument(name = "Handling message", skip_all)]
    fn handle_message(
        message: Message,
        writer: &mut T,
        output_dir: &str,
    ) -> Result<(), Box<dyn Error>> {
        write_to_output(writer, message.to_string().as_bytes())?;
        Self::store_data(message.data, writer, output_dir)?;
        Ok(())
    }

    #[tracing::instrument(name = "Saving data to output dir", skip_all)]
    fn store_data(
        message: MessagePayload,
        writer: &mut T,
        output_dir: &str,
    ) -> Result<(), Box<dyn Error>> {
        match message {
            MessagePayload::Image(data) => {
                let now = Utc::now();
                let timestamp = now.timestamp();
                let file_path = format!("{}/images/{}.png", output_dir, timestamp);
                save_file(&file_path, &data)?;
                write_to_output(
                    writer,
                    format!("Image saved to: {}\n", file_path).as_bytes(),
                )?;
            }
            MessagePayload::File(file_name, data) => {
                let file_path = format!("{}/files/{}", output_dir, file_name);
                save_file(&file_path, &data)?;
                write_to_output(writer, format!("File saved to: {}\n", file_path).as_bytes())?;
            }
            _ => {}
        }
        Ok(())
    }
}
