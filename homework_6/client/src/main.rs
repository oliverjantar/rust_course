mod args;
mod command;

use args::Args;
use chrono::Utc;
use clap::Parser;
use command::Command;
use shared::message_type::{receive_msg, send_msg, MessageType};
use shared::tracing::{get_subscriber, init_subscriber};
use std::io::Write;
use std::net::TcpStream;
use std::str::FromStr;
use std::{error::Error, thread};

fn main() {
    let args = Args::parse();

    let tracing_subscriber = get_subscriber("client".into(), "debug".into(), std::io::stdout);
    init_subscriber(tracing_subscriber);

    let output_writer = std::io::stdout();

    if let Err(e) = start(args, output_writer) {
        log_error(e);
    }
}

#[tracing::instrument(name = "Starting client", skip(writer))]
fn start<T>(args: Args, mut writer: T) -> Result<(), Box<dyn Error>>
where
    T: Write + Send + 'static,
{
    let server = format!("{}:{}", args.host, args.port);

    writer.write_all(b"Connecting to server on {server}...")?;

    let stream = TcpStream::connect(server)?;
    stream.set_nodelay(true)?;

    writer.write_all(b"Connected.")?;

    let _ = thread::spawn({
        let stream = stream.try_clone().unwrap();
        move || receive_messages(stream, writer, &args.output_dir)
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

fn receive_messages<T>(mut stream: TcpStream, mut writer: T, output_dir: &str)
where
    T: Write,
{
    while let Ok(message) = receive_msg(&mut stream) {
        if let Err(e) = handle_message(message, &mut writer, output_dir) {
            log_error(e)
        }
    }
}

#[tracing::instrument(name = "Received message", skip_all)]
pub fn handle_message<T>(
    message: MessageType,
    writer: &mut T,
    output_dir: &str,
) -> Result<(), Box<dyn Error>>
where
    T: Write,
{
    match message {
        MessageType::Text(text) => writer.write_all(text.as_bytes())?,
        MessageType::Image(data) => {
            writer.write_all(b"Receiving image...")?;
            let now = Utc::now();
            let timestamp = now.timestamp();
            let file_path = format!("{}/images/{}.png", output_dir, timestamp);
            save_file(&file_path, &data)?;
        }
        MessageType::File(file_name, data) => {
            writer.write_all(format!("Receiving {}", file_name).as_bytes())?;
            let file_path = format!("{}/files/{}", output_dir, file_name);
            save_file(&file_path, &data)?;
        }
    }

    Ok(())
}

fn save_file(path: &str, data: &[u8]) -> std::io::Result<()> {
    let path = std::path::Path::new(path);

    if let Some(dir_path) = path.parent() {
        if !dir_path.exists() {
            std::fs::create_dir_all(dir_path)?;
        }
    }

    let mut file = std::fs::File::create(path)?;
    file.write_all(data)?;
    Ok(())
}

fn log_error(e: Box<dyn Error>) {
    tracing::error!("Error: {e}");
}
