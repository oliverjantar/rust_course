use std::{
    error::Error,
    io::Write,
    net::{Ipv4Addr, TcpStream},
    str::FromStr,
};

use chrono::Utc;
use shared::message_type::{receive_msg, send_msg, MessageType};

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
    ) -> Result<(ClientSender, ClientReceiver<T>), Box<dyn Error>>
    where
        T: Write,
    {
        let server = format!("{}:{}", host, port);

        write_to_output(
            &mut writer,
            format!("Connecting to server on {}...", server).as_bytes(),
        )?;

        let stream = TcpStream::connect(server)?;
        stream.set_nodelay(true)?;

        write_to_output(&mut writer, b"Connected. You can now send messages.")?;
        let receiver_stream = stream.try_clone()?;
        let receiver = ClientReceiver::new(receiver_stream, writer, output_dir);
        let sender = ClientSender::new(stream);

        Ok((sender, receiver))
    }
}

pub struct ClientSender {
    stream: TcpStream,
}

impl ClientSender {
    fn new(stream: TcpStream) -> Self {
        ClientSender { stream }
    }

    pub fn start(mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let mut text = String::new();

            std::io::stdin().read_line(&mut text)?;

            let cmd = Command::from_str(text.trim())?;

            if cmd == Command::Quit {
                return Ok(());
            }

            let message = match cmd.try_into() {
                Ok(message) => message,
                Err(e) => {
                    log_error(e);
                    continue;
                }
            };

            send_msg(&message, &mut self.stream)?;
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
        while let Ok(message) = receive_msg(&mut self.stream) {
            if let Err(e) = Self::handle_message(message, &mut self.writer, &self.output_dir) {
                log_error(e)
            }
        }
    }

    #[tracing::instrument(name = "Handling message", skip_all)]
    fn handle_message(
        message: MessageType,
        writer: &mut T,
        output_dir: &str,
    ) -> Result<(), Box<dyn Error>> {
        match message {
            MessageType::Text(text) => write_to_output(writer, text.as_bytes())?,
            MessageType::Image(data) => {
                write_to_output(writer, b"Receiving image...")?;
                let now = Utc::now();
                let timestamp = now.timestamp();
                let file_path = format!("{}/images/{}.png", output_dir, timestamp);
                save_file(&file_path, &data)?;
                write_to_output(writer, format!("Image saved to: {}", file_path).as_bytes())?;
            }
            MessageType::File(file_name, data) => {
                write_to_output(writer, format!("Receiving {}", file_name).as_bytes())?;
                let file_path = format!("{}/files/{}", output_dir, file_name);
                save_file(&file_path, &data)?;
                write_to_output(writer, format!("File saved to: {}", file_path).as_bytes())?;
            }
        }
        writer.flush()?;
        Ok(())
    }
}
