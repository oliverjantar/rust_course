use crate::errors::MessageError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Main message struct that wraps the data and other metadata fields.
/// sender: the username of the sender
/// timestamp: when msg was created, not used at the moment but it will be useful for the frontend
/// data: the actual payload of the message
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub sender: Option<String>,
    pub timestamp: i64,
    pub data: MessagePayload,
}

impl Message {
    /// Creates a new message with the given username and data.
    pub fn new(data: MessagePayload) -> Self {
        let now = Utc::now();
        Message {
            sender: None,
            timestamp: now.timestamp(),
            data,
        }
    }

    /// Creates a new server info message with the given text.
    pub fn new_server_msg(text: &str) -> Self {
        let now = Utc::now();
        Message {
            data: MessagePayload::ServerInfo(text.to_owned()),
            sender: None,
            timestamp: now.timestamp(),
        }
    }

    pub fn set_from_user(&mut self, sender: &str) {
        self.sender = Some(sender.to_owned())
    }

    fn serialize(message: &Message) -> Result<Vec<u8>, MessageError> {
        bincode::serialize(message).map_err(MessageError::SerializeError)
    }

    fn deserialize(data: &[u8]) -> Result<Message, MessageError> {
        bincode::deserialize(data).map_err(MessageError::DeserializeError)
    }

    /// Sends the message to the given stream.
    #[tracing::instrument(name = "Sending message", skip(message, stream))]
    pub async fn send_msg<T>(message: &Message, stream: &mut T) -> Result<(), MessageError>
    where
        T: AsyncWrite + Unpin,
    {
        let serialized = Message::serialize(message)?;
        let length = serialized.len() as u32;

        _ = stream
            .write(&length.to_be_bytes())
            .await
            .map_err(MessageError::SendError)?;

        stream
            .write_all(&serialized)
            .await
            .map_err(MessageError::SendError)?;
        Ok(())
    }

    /// Receives a message from the given stream.
    pub async fn receive_msg<T>(stream: &mut T) -> Result<Message, MessageError>
    where
        T: AsyncRead + Unpin,
    {
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(MessageError::RecieveError)?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut buffer = vec![0u8; len];

        stream
            .read_exact(&mut buffer)
            .await
            .map_err(MessageError::RecieveError)?;

        let message = Message::deserialize(&buffer)?;

        Ok(message)
    }

    pub async fn send_active_users_msg<T>(
        stream: &mut T,
        active_users: usize,
    ) -> Result<(), MessageError>
    where
        T: AsyncWrite + Unpin,
    {
        let msg = Self::new_server_msg(&format!("Active users: {}", active_users));
        Message::send_msg(&msg, stream).await?;
        Ok(())
    }

    pub async fn handshake<T>(stream: &mut T, user: AuthUser) -> Result<Message, MessageError>
    where
        T: AsyncWrite + AsyncRead + Unpin,
    {
        let payload = MessagePayload::Login(user);
        let msg = Message::new(payload);

        Message::send_msg(&msg, stream).await?;
        let msg = Message::receive_msg(stream).await?;

        Ok(msg)
    }
}

/// Inner stuct that contains the data of the message.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum MessagePayload {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>),
    ServerInfo(String),
    Login(AuthUser),
    LoginResponse(AuthPayload),
}

impl MessagePayload {
    pub fn serialize_to_text(data: &MessagePayload) -> String {
        match data {
            MessagePayload::Text(text) => text.to_owned(),
            MessagePayload::Image(_) => "img sent".to_string(),
            MessagePayload::File(name, _) => format!("file sent: {name}"),
            MessagePayload::ServerInfo(_) => "".to_string(),
            MessagePayload::Login(_) => "".to_string(),
            MessagePayload::LoginResponse(_) => "".to_string(),
        }
    }
}
const ANONYMOUS: &str = "anonymous";
/// Formats the message based on the data type.
impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            MessagePayload::Text(text) => writeln!(
                f,
                "{}: {}",
                self.sender.as_ref().unwrap_or(&ANONYMOUS.to_string()),
                text
            )?,
            MessagePayload::Image(_) => writeln!(
                f,
                "{} sent an image",
                self.sender.as_ref().unwrap_or(&ANONYMOUS.to_string()),
            )?,
            MessagePayload::File(filename, _) => writeln!(
                f,
                "{} sent a file {}",
                self.sender.as_ref().unwrap_or(&ANONYMOUS.to_string()),
                filename
            )?,
            MessagePayload::ServerInfo(text) => writeln!(f, "--      {}      --", text)?,
            MessagePayload::Login(_) => writeln!(f, "Login payload")?, //This won't be ever displayed in the client output
            MessagePayload::LoginResponse(data) => writeln!(f, "{}", data)?,
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct AuthUser {
    pub name: String,
    pub password: String,
}
impl AuthUser {
    pub fn new(name: &str, password: &str) -> Self {
        Self {
            name: name.to_owned(),
            password: password.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct AuthPayload {
    is_ok: bool,
    message: Option<AuthMessage>,
    err: Option<AuthError>,
}

impl AuthPayload {
    pub fn new_login() -> Self {
        Self {
            is_ok: true,
            message: Some(AuthMessage::LoginSuccessful),
            err: None,
        }
    }

    pub fn new_register() -> Self {
        Self {
            is_ok: true,
            message: Some(AuthMessage::UserRegistered),
            err: None,
        }
    }
    pub fn new_error() -> Self {
        Self {
            is_ok: false,
            message: None,
            err: Some(AuthError::IncorrectPassword),
        }
    }
}

impl AuthPayload {
    pub fn is_success(&self) -> bool {
        self.is_ok
    }
}
impl Display for AuthPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_ok {
            let message = self
                .message
                .as_ref()
                .unwrap_or(&AuthMessage::LoginSuccessful);
            match message {
                AuthMessage::LoginSuccessful => writeln!(f, "Login was successful.")?,
                AuthMessage::UserRegistered => writeln!(f, "You were successfully registered.")?,
            }
        } else {
            writeln!(f, "Login failed, incorrect password.")?
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum AuthMessage {
    LoginSuccessful,
    UserRegistered,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum AuthError {
    IncorrectPassword,
}
