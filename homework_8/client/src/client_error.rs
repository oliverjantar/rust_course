use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Failed to write to output: {0}")]
    Write(#[source] std::io::Error),
    #[error("Failed to encrypt message")]
    EncryptMessage,
    #[error("Failed to decrypt message. {}",.0.as_deref().unwrap_or("No additional info"))]
    DecryptMessage(Option<String>),
    #[error("Failed to create directory for output files. {0}")]
    CreateDir(#[source] io::Error),
    #[error("Failed to create file in output directory. {0}")]
    CreateFile(#[source] io::Error),
    #[error("Failed to write to file in output directory. {0}")]
    WriteToFile(#[source] io::Error),
    #[error("Couldn't read content of a file. {0}")]
    ReadFromFile(#[source] io::Error),
    #[error("Cannot send the file, file does not exist")]
    FileNotExists,
    #[error("Failed to convert image to png format")]
    ConvertImagePng,
    #[error("Cannot open image or image does not exist. {0}")]
    OpenImage(#[source] io::Error),
    #[error("Invalid command to transfrom into Message")]
    InvalidCommand,
}
