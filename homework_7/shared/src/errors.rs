use bincode::Error as BincodeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Failed to serialize message")]
    SerializeError(BincodeError),
    #[error("Failed to deserialize message")]
    DeserializeError(BincodeError),
    #[error("Failed to send message")]
    SendError(std::io::Error),
    #[error("Failed to receive message")]
    RecieveError(std::io::Error),
}

#[derive(Debug, Error)]
pub enum TracingErrors {
    #[error("Failed to create directory for logs")]
    CreateDirError(std::io::Error),
    #[error("Failed to create log file")]
    CreateLogFileError(std::io::Error),
    #[error("Failed to setup tracing")]
    SetupTracingError(String),
}
