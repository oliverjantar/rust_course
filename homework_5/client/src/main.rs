use std::net::TcpStream;
use std::str::FromStr;
use std::{error::Error, thread};

use shared::message_type::{receive_msg, send_msg, MessageType};

fn main() {
    let address = std::env::args()
        .nth(1)
        .unwrap_or("127.0.0.1:11111".to_string());

    if let Err(e) = start(&address) {
        eprintln!("Error from server: {e}");
    }
}

fn start(address: &str) -> Result<(), Box<dyn Error>> {
    println!("Connecting to server on {address}...");

    let stream = TcpStream::connect(address)?;
    stream.set_nodelay(true)?;

    let _ = thread::spawn({
        let stream = stream.try_clone().unwrap();
        move || receive_messages(stream)
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

        let message = match cmd.into_message() {
            Ok(message) => message,
            Err(e) => {
                eprintln!("Error: {e}");
                continue;
            }
        };

        send_msg(&message, &mut stream)?;
        println!("Message sent.");
    }
}

fn receive_messages(mut stream: TcpStream) {
    while let Ok(message) = receive_msg(&mut stream) {
        if let Err(e) = message.handle_message() {
            eprintln!("Error while processing message: {}", e)
        }
    }
}

#[derive(PartialEq)]
enum Command {
    Text(String),
    File(String),
    Image(String),
    Quit,
}

impl Command {
    fn into_message(self) -> Result<MessageType, Box<dyn Error>> {
        match self {
            Command::Text(text) => Ok(MessageType::Text(text.to_owned())),
            Command::File(path) => MessageType::get_file(&path),
            Command::Image(path) => MessageType::get_image(&path),
            Command::Quit => Err("No message to send.".into()),
        }
    }
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, ' ');
        let first_arg = parts.next().unwrap_or("");
        let second_arg = parts.next().unwrap_or("");

        match first_arg {
            ".file" => Ok(Command::File(second_arg.to_string())),
            ".image" => Ok(Command::Image(second_arg.to_string())),
            ".quit" => Ok(Command::Quit),
            _ => Ok(Command::Text(s.to_string())),
        }
    }
}
