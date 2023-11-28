use bincode::Error as BincodeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Failed to serialize message. {0}")]
    SerializeError(#[source] BincodeError),
    #[error("Failed to deserialize message. {0}")]
    DeserializeError(#[source] BincodeError),
    #[error("Failed to send message. {0}")]
    SendError(#[source] std::io::Error),
    #[error("Failed to receive message. {0}")]
    RecieveError(#[source] std::io::Error),
}

#[derive(Debug, Error)]
pub enum TracingErrors {
    #[error("Failed to create directory for logs. {0}")]
    CreateDirError(#[source] std::io::Error),
    #[error("Failed to create a log file. {0}")]
    CreateLogFileError(#[source] std::io::Error),
    #[error("Failed to setup tracing for client. {0}")]
    SetupTracingError(String),
}
