use std::io;

use thiserror::Error;
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Failed to write to output")]
    Write(std::io::Error),
    #[error("Failed to parse command")]
    CommandParse(String),
    #[error("Failed to encrypt message")]
    EncryptMessage,
    #[error("Failed to decrypt message")]
    DecryptMessage(Option<String>),
    #[error("Failed to create directory for output files")]
    CreateDir(io::Error),
    #[error("Failed to create file in output directory")]
    CreateFile(io::Error),
    #[error("Failed to write to file in output directory")]
    WriteToFile(io::Error),
    #[error("Couldn't read content of a file")]
    ReadFromFile,
    #[error("File does not exist")]
    FileNotExists,
    #[error("User does not have permissions to the file")]
    FilePermissions,
    #[error("Failed to convert image to png format")]
    ConvertImagePng,
    #[error("Cannot open image or image does not exist")]
    OpenImage(io::Error),
    #[error("Invalid command to transfrom into Message")]
    InvalidCommand,
}
