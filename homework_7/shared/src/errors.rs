use std::{error::Error, fmt::Display};

use bincode::Error as BincodeError;

#[derive(Debug)]
pub enum MessageError {
    SerializeError(BincodeError),
    DeserializeError(BincodeError),
    SendError(std::io::Error),
    RecieveError(std::io::Error),
}

impl Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageError::SerializeError(err) => write!(f, "Failed to serialize message: {}", err),
            MessageError::DeserializeError(err) => {
                write!(f, "Failed to deserialize message: {}", err)
            }
            MessageError::SendError(err) => write!(f, "Failed to send message: {}", err),
            MessageError::RecieveError(err) => write!(f, "Failed to receive message: {}", err),
        }
    }
}

impl Error for MessageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MessageError::SerializeError(err) => Some(err),
            MessageError::DeserializeError(err) => Some(err),
            MessageError::SendError(err) => Some(err),
            MessageError::RecieveError(err) => Some(err),
        }
    }
}

#[derive(Debug)]
pub enum TracingErrors {
    CreateDirError(std::io::Error),
    CreateLogFileError(std::io::Error),
    SetupTracingError(String),
}

impl Display for TracingErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TracingErrors::CreateDirError(e) => {
                write!(f, "Failed to create directory for logs: {}", e)
            }
            TracingErrors::CreateLogFileError(e) => {
                write!(f, "Failed to create log file: {}", e)
            }
            TracingErrors::SetupTracingError(e) => write!(f, "Failed to setup tracing: {}", e),
        }
    }
}

impl Error for TracingErrors {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TracingErrors::CreateDirError(e) => Some(e),
            TracingErrors::CreateLogFileError(e) => Some(e),
            TracingErrors::SetupTracingError(_) => None,
        }
    }
}
