mod args;
mod command;
mod utils;

use args::Args;
use chrono::Utc;
use clap::Parser;
use command::Command;
use shared::message_type::{receive_msg, send_msg, MessageType};
use shared::tracing::{create_log_file, get_subscriber, init_subscriber};
use std::io::Write;
use std::net::TcpStream;
use std::str::FromStr;
use std::{error::Error, thread};
use utils::save_file;

fn main() {
    let args = Args::parse();
    setup_tracing(&args.logs_dir);

    let output_writer = std::io::stdout();

    if let Err(e) = start(args, output_writer) {
        log_error(e);
    }
}

fn setup_tracing(logs_dir: &str) {
    let log_file = create_log_file(logs_dir, "client").expect("Failed to create log file");

    let tracing_subscriber = get_subscriber("client".into(), "debug".into(), log_file); //std::io::stdout);
    init_subscriber(tracing_subscriber);
}

#[tracing::instrument(name = "Starting client", skip(writer))]
fn start<T>(args: Args, mut writer: T) -> Result<(), Box<dyn Error>>
where
    T: Write + Send + 'static,
{
    let server = format!("{}:{}", args.host, args.port);

    write_to_output(
        &mut writer,
        format!("Connecting to server on {}...", server).as_bytes(),
    )?;

    let stream = TcpStream::connect(server)?;
    stream.set_nodelay(true)?;

    write_to_output(&mut writer, b"Connected. You can now send messages.")?;

    let _ = thread::spawn({
        let stream = stream.try_clone().unwrap();
        let receiver = ClientReceiver::new(writer, &args.output_dir);
        move || receiver.start(stream)
    });

    send_messages(stream)?;

    Ok(())
}

fn send_messages(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
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

        send_msg(&message, &mut stream)?;
    }
}

struct ClientReceiver<T> {
    writer: T,
    output_dir: String,
}

impl<T> ClientReceiver<T>
where
    T: Write,
{
    pub fn new(writer: T, output_dir: &str) -> Self {
        Self {
            writer,
            output_dir: output_dir.to_string(),
        }
    }

    pub fn start(mut self, mut stream: TcpStream) {
        while let Ok(message) = receive_msg(&mut stream) {
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

fn log_error(e: Box<dyn Error>) {
    tracing::error!("Error: {e}");
    eprintln!("Error: {e}");
}

fn write_to_output<T>(writer: &mut T, buf: &[u8]) -> Result<(), std::io::Error>
where
    T: Write,
{
    writer.write_all(buf)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}
