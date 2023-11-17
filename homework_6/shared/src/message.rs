use std::{
    error::Error,
    fmt::Display,
    io::{Read, Write},
    net::TcpStream,
};

use bincode::Error as BincodeError;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>), // Filename and its content as bytes
    ServerInfo(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessagePayload {
    pub sender: String,
    pub timestamp: i64,
    pub data: MessageType,
}

impl MessagePayload {
    pub fn new_server_msg(text: &str) -> Self {
        let now = Utc::now();

        MessagePayload {
            data: MessageType::ServerInfo(text.to_owned()),
            sender: "server".to_owned(),
            timestamp: now.timestamp(),
        }
    }
    pub fn serialize(message: &MessagePayload) -> Result<Vec<u8>, BincodeError> {
        bincode::serialize(message)
    }

    pub fn deserialize(data: &[u8]) -> Result<MessagePayload, BincodeError> {
        bincode::deserialize(data)
    }

    #[tracing::instrument(name = "Sending message", skip(message, stream))]
    pub fn send_msg(
        message: &MessagePayload,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn Error>> {
        let serialized = MessagePayload::serialize(message)?;
        let length = serialized.len() as u32;

        _ = stream.write(&length.to_be_bytes())?;

        stream.write_all(&serialized)?;

        Ok(())
    }

    pub fn receive_msg(stream: &mut TcpStream) -> Result<MessagePayload, Box<dyn Error>> {
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes)?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut buffer = vec![0u8; len];

        stream.read_exact(&mut buffer)?;

        let message = MessagePayload::deserialize(&buffer)?;

        Ok(message)
    }

    pub fn send_active_users_msg(
        stream: &mut TcpStream,
        active_users: usize,
    ) -> Result<(), Box<dyn Error>> {
        let msg = Self::new_server_msg(&format!("Active users: {}", active_users - 1));
        MessagePayload::send_msg(&msg, stream)?;
        Ok(())
    }

    pub fn send_new_user_msg(stream: &mut TcpStream, username: &str) -> Result<(), Box<dyn Error>> {
        let msg = Self::new_server_msg(&format!("New user connected: {}", username));
        MessagePayload::send_msg(&msg, stream)?;
        Ok(())
    }
}

impl Display for MessagePayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            MessageType::Text(text) => writeln!(f, "{}: {}", self.sender, text)?,
            MessageType::Image(_) => writeln!(f, "{} sent an image", self.sender)?,
            MessageType::File(filename, _) => {
                writeln!(f, "{} sent a file {}", self.sender, filename)?
            }
            MessageType::ServerInfo(text) => writeln!(f, "--      {}      --", text)?,
        }
        Ok(())
    }
}
