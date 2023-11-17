use std::{
    error::Error,
    io::Write,
    net::{Ipv4Addr, TcpStream},
    str::FromStr,
};

use chrono::Utc;
use shared::message::{MessagePayload, MessageType};

use crate::{
    command::Command,
    utils::{log_error, save_file, write_to_output},
};

pub struct Client;

impl Client {
    pub fn connect<T>(
        host: Ipv4Addr,
        port: u32,
        mut writer: T,
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

        let stream = TcpStream::connect(server)?;
        stream.set_nodelay(true)?;

        write_to_output(&mut writer, b"Connected. You can now send messages.\n")?;
        let receiver_stream = stream.try_clone()?;
        let receiver = ClientReceiver::new(receiver_stream, writer, output_dir);
        let sender = ClientSender::new(stream, username.to_owned());

        Ok((sender, receiver))
    }
}

pub struct ClientSender {
    stream: TcpStream,
    username: String,
}

impl ClientSender {
    fn new(stream: TcpStream, username: String) -> Self {
        ClientSender { stream, username }
    }

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
            let now = Utc::now();
            let message = MessagePayload {
                sender: self.username.clone(),
                timestamp: now.timestamp(),
                data,
            };

            MessagePayload::send_msg(&message, &mut self.stream)?;
        }
    }
}

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
        while let Ok(message) = MessagePayload::receive_msg(&mut self.stream) {
            if let Err(e) = Self::handle_message(message, &mut self.writer, &self.output_dir) {
                log_error(e)
            }
        }
    }

    #[tracing::instrument(name = "Handling message", skip_all)]
    fn handle_message(
        message: MessagePayload,
        writer: &mut T,
        output_dir: &str,
    ) -> Result<(), Box<dyn Error>> {
        write_to_output(writer, message.to_string().as_bytes())?;
        Self::store_data(message.data, writer, output_dir)?;
        Ok(())
    }

    fn store_data(
        message: MessageType,
        writer: &mut T,
        output_dir: &str,
    ) -> Result<(), Box<dyn Error>> {
        match message {
            MessageType::Image(data) => {
                let now = Utc::now();
                let timestamp = now.timestamp();
                let file_path = format!("{}/images/{}.png", output_dir, timestamp);
                save_file(&file_path, &data)?;
                write_to_output(writer, format!("Image saved to: {}", file_path).as_bytes())?;
            }
            MessageType::File(file_name, data) => {
                let file_path = format!("{}/files/{}", output_dir, file_name);
                save_file(&file_path, &data)?;
                write_to_output(writer, format!("File saved to: {}", file_path).as_bytes())?;
            }
            _ => {}
        }
        Ok(())
    }
}
