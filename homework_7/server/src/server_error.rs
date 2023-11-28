use shared::{errors::MessageError, message::Message};
use std::{io, net::SocketAddr, sync::mpsc::SendError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Cannot set non-blocking tcp listener.")]
    NonblockingListener,
    #[error("Cannot bind to the address. {0}")]
    BindError(#[source] io::Error),
    #[error("Error from a broadcasting thread.")]
    BroadcastThreadError,
    #[error("Failed to get peer address: {0}")]
    PeerAddressError(#[source] io::Error),
    #[error("Failed to clone TCP stream: {0}")]
    StreamCloneError(#[source] io::Error),
    #[error("Failed to lock clients map")]
    ClientsLockError,
    #[error("Failed to send message: {0}")]
    SendMessageError(#[source] MessageError),
    #[error("Channel send error: {0}")]
    ChannelSendError(#[source] SendError<(SocketAddr, Message)>),
}
