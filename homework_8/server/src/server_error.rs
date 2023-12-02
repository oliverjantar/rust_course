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
}
