use std::{
    error::Error,
    io::{Read, Write},
    net::TcpStream,
};

use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeError;

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>), // Filename and its content as bytes
}

impl MessageType {
    pub fn serialize(message: &MessageType) -> Result<String, SerdeError> {
        serde_json::to_string(message)
    }

    pub fn deserialize(data: &[u8]) -> Result<MessageType, SerdeError> {
        serde_json::from_slice(data)
    }
}

pub fn send_msg(message: &MessageType, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    let serialized = MessageType::serialize(message)?;
    let length = serialized.len() as u32;

    _ = stream.write(&length.to_be_bytes())?;

    stream.write_all(serialized.as_bytes())?;

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
