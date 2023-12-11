use flume::SendError;
use shared::{errors::MessageError, message::Message};
use std::{io, net::SocketAddr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Cannot bind to the address. {0}")]
    Bind(#[source] io::Error),
    #[error("Failed to send message: {0}")]
    SendMessage(#[source] MessageError),
    #[error("Channel send error: {0}")]
    ChannelSend(#[source] SendError<(SocketAddr, Message)>),
    #[error("Failed to store message")]
    StoreMessage,
    #[error("Failed to store user")]
    StoreUser,
    #[error("Failed to get user")]
    GetUser,
    #[error("Failed to get messages")]
    GetMessages,
    #[error("Failed to delete user")]
    DeleteUser,
    #[error("Failed to decode password")]
    PasswordDecode,
    #[error("Failed to create user")]
    CreateUser,
}
