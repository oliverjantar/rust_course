mod args;
mod command;

use args::Args;
use clap::Parser;
use command::Command;
use shared::message_type::{receive_msg, send_msg};

use std::net::TcpStream;
use std::str::FromStr;
use std::{error::Error, thread};

fn main() {
    let args = Args::parse();

    if let Err(e) = start(args) {
        log_error(e);
    }
}

fn start(args: Args) -> Result<(), Box<dyn Error>> {
    let server = format!("{}:{}", args.host, args.port);
    println!("Connecting to server on {server}...");

    let stream = TcpStream::connect(server)?;
    stream.set_nodelay(true)?;

    let _ = thread::spawn({
        let stream = stream.try_clone().unwrap();
        move || receive_messages(stream, &args.output_dir)
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
        println!("Message sent.");
    }
}

fn receive_messages(mut stream: TcpStream, output_dir: &str) {
    while let Ok(message) = receive_msg(&mut stream) {
        if let Err(e) = message.handle_message(output_dir) {
            log_error(e)
        }
    }
}

fn log_error(e: Box<dyn Error>) {
    eprintln!("Error: {e}");
}
