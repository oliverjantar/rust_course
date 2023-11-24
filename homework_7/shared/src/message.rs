use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    io::{Read, Write},
    net::TcpStream,
};

use crate::errors::MessageError;

/// Main message struct that wraps the data and other metadata fields.
/// sender: the username of the sender
/// timestamp: when msg was created, not used at the moment but it will be useful for the frontend
/// data: the actual payload of the message
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub sender: String,
    pub timestamp: i64,
    pub data: MessagePayload,
}

impl Message {
    /// Creates a new message with the given username and data.
    pub fn new(username: &str, data: MessagePayload) -> Self {
        let now = Utc::now();
        Message {
            sender: username.to_owned(),
            timestamp: now.timestamp(),
            data,
        }
    }

    /// Creates a new server info message with the given text.
    pub fn new_server_msg(text: &str) -> Self {
        let now = Utc::now();
        Message {
            data: MessagePayload::ServerInfo(text.to_owned()),
            sender: "server".to_owned(),
            timestamp: now.timestamp(),
        }
    }

    fn serialize(message: &Message) -> Result<Vec<u8>, MessageError> {
        bincode::serialize(message).map_err(MessageError::SerializeError)
    }

    fn deserialize(data: &[u8]) -> Result<Message, MessageError> {
        bincode::deserialize(data).map_err(MessageError::DeserializeError)
    }

    /// Sends the message to the given stream.
    #[tracing::instrument(name = "Sending message", skip(message, stream))]
    pub fn send_msg(message: &Message, stream: &mut TcpStream) -> Result<(), MessageError> {
        let serialized = Message::serialize(message)?;
        let length = serialized.len() as u32;

        _ = stream
            .write(&length.to_be_bytes())
            .map_err(MessageError::SendError)?;

        stream
            .write_all(&serialized)
            .map_err(MessageError::SendError)?;

        Ok(())
    }

    /// Receives a message from the given stream.
    pub fn receive_msg(stream: &mut TcpStream) -> Result<Message, MessageError> {
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .map_err(MessageError::RecieveError)?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut buffer = vec![0u8; len];

        stream
            .read_exact(&mut buffer)
            .map_err(MessageError::RecieveError)?;

        let message = Message::deserialize(&buffer)?;

        Ok(message)
    }

    pub fn send_active_users_msg(
        stream: &mut TcpStream,
        active_users: usize,
    ) -> Result<(), MessageError> {
        let msg = Self::new_server_msg(&format!("Active users: {}", active_users - 1));
        Message::send_msg(&msg, stream)?;
        Ok(())
    }

    pub fn send_new_user_msg(stream: &mut TcpStream, username: &str) -> Result<(), MessageError> {
        let msg = Self::new_server_msg(&format!("New user connected: {}", username));
        Message::send_msg(&msg, stream)?;
        Ok(())
    }
}

/// Inner stuct that contains the data of the message.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum MessagePayload {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>),
    ServerInfo(String),
}

/// Formats the message based on the data type.
impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            MessagePayload::Text(text) => writeln!(f, "{}: {}", self.sender, text)?,
            MessagePayload::Image(_) => writeln!(f, "{} sent an image", self.sender)?,
            MessagePayload::File(filename, _) => {
                writeln!(f, "{} sent a file {}", self.sender, filename)?
            }
            MessagePayload::ServerInfo(text) => writeln!(f, "--      {}      --", text)?,
        }
        Ok(())
    }
}
