use std::{
    error::Error,
    io::{Read, Write},
    net::TcpStream,
};

use bincode::Error as SerdeError;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>), // Filename and its content as bytes
}

impl MessageType {
    pub fn serialize(message: &MessageType) -> Result<Vec<u8>, SerdeError> {
        bincode::serialize(message)
    }

    pub fn deserialize(data: &[u8]) -> Result<MessageType, SerdeError> {
        bincode::deserialize(data)
    }

    pub fn handle_message(self) -> Result<(), Box<dyn Error>> {
        match self {
            MessageType::Text(text) => println!("{}", text),
            MessageType::Image(data) => {
                println!("Receiving image...");
                let now = Utc::now();
                let timestamp = now.timestamp();
                let file_path = format!("./images/{}.png", timestamp);
                MessageType::save_file(&file_path, &data)?;
            }
            MessageType::File(file_name, data) => {
                println!("Receiving {file_name}");
                let file_path = format!("./files/{}", file_name);
                MessageType::save_file(&file_path, &data)?;
            }
        }

        Ok(())
    }

    pub fn get_file(path: &str) -> Result<MessageType, Box<dyn Error>> {
        let path = std::path::Path::new(path);

        let file_name_os = path.file_name();

        let file_name = match file_name_os {
            Some(file_name) => file_name.to_string_lossy(),
            None => return Err("File does not exist.".into()),
        };

        let bytes = std::fs::read(path)?;

        Ok(MessageType::File(file_name.to_string(), bytes))
    }

    pub fn get_image(path: &str) -> Result<MessageType, Box<dyn Error>> {
        let bytes = std::fs::read(path)?;

        Ok(MessageType::Image(bytes))
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
}

pub fn send_msg(message: &MessageType, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    let serialized = MessageType::serialize(message)?;
    let length = serialized.len() as u32;

    _ = stream.write(&length.to_be_bytes())?;

    stream.write_all(&serialized)?;

    Ok(())
}

pub fn receive_msg(stream: &mut TcpStream) -> Result<MessageType, Box<dyn Error>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;

    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];

    stream.read_exact(&mut buffer)?;

    let message = MessageType::deserialize(&buffer)?;

    Ok(message)
}
