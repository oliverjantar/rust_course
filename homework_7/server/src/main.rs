mod args;

use anyhow::{bail, Result};
use args::Args;
use clap::Parser;
use shared::errors::MessageError;
use shared::message::Message;
use shared::tracing::{get_subscriber, init_subscriber};
use std::io;
use std::sync::mpsc::{self, SendError};
use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};
use thiserror::Error;

fn main() {
    let args = Args::parse();

    // Setup tracing, default output is stdout.
    let tracing_subscriber = get_subscriber("server".into(), "debug".into(), std::io::stdout);
    if let Err(e) = init_subscriber(tracing_subscriber) {
        tracing::error!("Error while setting up server. {e}");
    }

    if let Err(e) = start(args) {
        tracing::error!("Error while running server. {e}");
    }
}

/// Starts the server. It will listen for incoming connections and spawn a new thread for each connection.
/// In a separate thread runs a broadcasting function that will send messages to all connected clients.
fn start(args: Args) -> Result<()> {
    let server = format!("{}:{}", args.host, args.port);
    tracing::info!("Starting server on address {server}...");

    let listener = TcpListener::bind(server).map_err(ServerError::BindError)?;

    if listener.set_nonblocking(true).is_err() {
        bail!(ServerError::NonblockingListener);
    }
    let (sender, receiver) = mpsc::channel();

    let clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));

    let broadcast_handle = thread::spawn({
        let clients = clients.clone();
        || broadcast_messages(clients, receiver)
    });

    #[allow(clippy::type_complexity)]
    let (error_sender, error_receiver): (Sender<ServerError>, Receiver<ServerError>) =
        mpsc::channel();

    thread::spawn(move || {
        for received_error in error_receiver {
            tracing::error!("Error in connection handling: {}", received_error);
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                thread::spawn({
                    let sender = sender.clone();
                    let clients = clients.clone();
                    let error_sender = error_sender.clone();
                    move || {
                        if let Err(e) = handle_connection(s, sender, clients) {
                            let _ = error_sender.send(e);
                        }
                    }
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => tracing::error!("Encountered network error from Tcp stream: {e}"),
        }
    }

    if broadcast_handle.join().is_err() {
        bail!(ServerError::BroadcastThreadError)
    }

    Ok(())
}

/// Handles a connection from a client.
/// In a loop it will listen for incoming messages and send them to the broadcasting thread using chanel.
fn handle_connection(
    mut stream: TcpStream,
    sender: Sender<(SocketAddr, Message)>,
    clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
) -> Result<(), ServerError> {
    let addr = stream.peer_addr().map_err(ServerError::PeerAddressError)?;

    clients
        .lock()
        .map_err(|_| ServerError::ClientsLockError)?
        .insert(
            addr,
            stream.try_clone().map_err(ServerError::StreamCloneError)?,
        );

    tracing::info!("New connection from: {addr}");

    let clients_count = clients
        .lock()
        .map_err(|_| ServerError::ClientsLockError)?
        .len();

    Message::send_active_users_msg(&mut stream, clients_count)
        .map_err(ServerError::SendMessageError)?;

    while let Ok(message) = Message::receive_msg(&mut stream) {
        tracing::info!("New message from: {addr}");
        sender
            .send((addr, message))
            .map_err(ServerError::ChannelSendError)?;
    }

    // If the client disconnects we remove it from the list of connected clients.
    remove_client(&clients, &addr);
    Ok(())
}

/// Broadcasts messages to all connected clients.
/// If a client is disconnected it will be removed from the list of connected clients.
fn broadcast_messages(
    clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
    receiver: Receiver<(SocketAddr, Message)>,
) {
    while let Ok((ip_addr, message)) = receiver.recv() {
        let mut clients_iter = clients.lock().expect("Failed to lock clients map");

        let clients_to_remove: Vec<SocketAddr> = clients_iter
            .iter_mut()
            .filter(|(client_addr, _)| **client_addr != ip_addr) // Filter out the client with ip_addr
            .filter_map(|(client_addr, stream)| {
                if let Err(e) = Message::send_msg(&message, stream) {
                    tracing::error!(
                        "Error while broadcasting message to client {client_addr}. Error: {e}"
                    );
                    Some(*client_addr)
                } else {
                    None
                }
            })
            .collect();

        clients_to_remove.iter().for_each(|&addr| {
            remove_client(&clients, &addr);
        });
    }
}

fn remove_client(clients: &Arc<Mutex<HashMap<SocketAddr, TcpStream>>>, ip_addr: &SocketAddr) {
    tracing::info!("Removing client from list {ip_addr}");
    clients
        .lock()
        .expect("Failed to lock clients map")
        .remove(ip_addr);
}

#[derive(Debug, Error)]
enum ServerError {
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
