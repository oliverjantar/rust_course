use std::{error::Error, str::FromStr};

use shared::message_type::MessageType;

#[derive(PartialEq)]
pub enum Command {
    Text(String),
    File(String),
    Image(String),
    Quit,
}

impl TryFrom<Command> for MessageType {
    type Error = Box<dyn Error>;

    fn try_from(value: Command) -> Result<Self, Self::Error> {
        match value {
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
