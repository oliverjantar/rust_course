use std::net::TcpStream;
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

    let handle = thread::spawn({
        let stream = stream.try_clone().unwrap();
        move || receive_messages(stream)
    });

    send_messages(stream)?;

    _ = handle.join();

    Ok(())
}

fn send_messages(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    loop {
        let mut text = String::new();

        std::io::stdin().read_line(&mut text)?;

        println!("Sending message...");

        let message = MessageType::Text(text);
        send_msg(&message, &mut stream)?;
    }
}

fn receive_messages(mut stream: TcpStream) {
    while let Ok(message) = receive_msg(&mut stream) {
        println!("Received message: {:?}", message);
    }
}
