use crate::{
    client_error::ClientError,
    utils::{get_file, get_image},
};
use shared::message::MessagePayload;
use std::str::FromStr;

/// User commands.
#[derive(PartialEq)]
pub enum Command {
    Text(String),
    File(String),
    Image(String),
    Quit,
}

impl Command {
    pub async fn into_message(self) -> Result<MessagePayload, ClientError> {
        match self {
            Command::Text(text) => Ok(MessagePayload::Text(text.to_owned())),
            Command::File(path) => get_file_message(&path).await,
            Command::Image(path) => get_image_message(&path).await,
            Command::Quit => Err(ClientError::InvalidCommand),
        }
    }
}
/*
impl TryFrom<Command> for MessagePayload {
    type Error = ClientError;

    fn try_from(value: Command) -> Result<Self, Self::Error> {
        match value {
            Command::Text(text) => Ok(MessagePayload::Text(text.to_owned())),
            Command::File(path) => get_file_message(&path).await,
            Command::Image(path) => get_image_message(&path).await,
            Command::Quit => Err(ClientError::InvalidCommand),
        }
    }
}
*/

impl FromStr for Command {
    type Err = ClientError;

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

async fn get_file_message(path: &str) -> Result<MessagePayload, ClientError> {
    let (name, data) = get_file(path).await?;
    Ok(MessagePayload::File(name, data))
}

async fn get_image_message(path: &str) -> Result<MessagePayload, ClientError> {
    let data = get_image(path).await?;
    Ok(MessagePayload::Image(data))
}
