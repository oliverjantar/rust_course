use std::{
    error::Error,
    io::{Cursor, Read, Write},
    net::TcpStream,
};

use bincode::Error as SerdeError;
use image::io::Reader as ImageReader;
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
        let bytes = match path.ends_with(".png") {
            true => std::fs::read(path)?,
            false => MessageType::convert_to_png(path)?,
        };
        Ok(MessageType::Image(bytes))
    }

    fn convert_to_png(path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut bytes = vec![];

        let img = ImageReader::open(path)?.decode()?;

        img.write_to(&mut Cursor::new(&mut bytes), image::ImageOutputFormat::Png)?;
        Ok(bytes)
    }
}
#[tracing::instrument(name = "Sending message", skip(message, stream))]
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
