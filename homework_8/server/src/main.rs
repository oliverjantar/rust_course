mod args;
mod server_error;

use anyhow::Result;
use args::Args;
use clap::Parser;
use flume::{Receiver, Sender};
use futures::stream::{self, StreamExt};
use server_error::ServerError;
use shared::message::Message;
use shared::tracing::{get_subscriber, init_subscriber};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Setup tracing, default output is stdout.
    let tracing_subscriber = get_subscriber("server".into(), "debug".into(), std::io::stdout);
    if let Err(e) = init_subscriber(tracing_subscriber) {
        tracing::error!("Error while setting up server. {e}");
    }

    if let Err(e) = start(args).await {
        tracing::error!("Error while running server. {e}");
    }
}

/// Starts the server. It will listen for incoming connections and spawn a new thread for each connection.
/// In a separate thread runs a broadcasting function that will send messages to all connected clients.
async fn start(args: Args) -> Result<()> {
    let server = format!("{}:{}", args.host, args.port);
    tracing::info!("Starting server on address {server}...");

    let listener = TcpListener::bind(server).await.map_err(ServerError::Bind)?;

    let (sender, receiver) = flume::unbounded();

    let clients: Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let _broadcast_handle = tokio::spawn({
        let clients = clients.clone();
        broadcast_messages(clients, receiver)
    });

    loop {
        match listener.accept().await {
            Ok((stream, address)) => {
                let sender = sender.clone();
                let clients = clients.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, address, sender, clients).await {
                        tracing::error!("Error in connection handling: {}", e);
                    }
                });
            }
            Err(e) => tracing::error!("Encountered network error from Tcp stream: {e}"),
        }
    }
}

/// Handles a connection from a client.
/// In a loop it will listen for incoming messages and send them to the broadcasting thread using chanel.
async fn handle_connection(
    stream: TcpStream,
    address: SocketAddr,
    sender: Sender<(SocketAddr, Message)>,
    clients: Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>>,
) -> Result<(), ServerError> {
    let (mut read_half, mut write_half) = stream.into_split();

    tracing::info!("New connection from: {address}");

    let clients_count = clients.lock().await.len();

    Message::send_active_users_msg(&mut write_half, clients_count)
        .await
        .map_err(ServerError::SendMessage)?;

    clients.lock().await.insert(address, write_half);

    while let Ok(message) = Message::receive_msg(&mut read_half).await {
        tracing::info!("New message from: {address}");
        sender
            .send((address, message))
            .map_err(ServerError::ChannelSend)?;
    }

    // If the client disconnects we remove it from the list of connected clients.
    remove_client(&clients, &address).await;
    Ok(())
}

/// Broadcasts messages to all connected clients.
/// If a client is disconnected it will be removed from the list of connected clients.
async fn broadcast_messages(
    clients: Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>>,
    receiver: Receiver<(SocketAddr, Message)>,
) {
    while let Ok((ip_addr, ref message)) = receiver.recv() {
        let mut clients_iter = clients.lock().await;

        let clients_to_remove: Vec<SocketAddr> = stream::iter(
            clients_iter
                .iter_mut()
                .filter(|(client_addr, _)| **client_addr != ip_addr), // Filter out the client that sent the message
        )
        .filter_map(|(client_addr, stream)| async move {
            tracing::debug!("Sending message to {client_addr}");
            if let Err(e) = Message::send_msg(message, stream).await {
                tracing::error!(
                    "Error while broadcasting message to client {client_addr}. Error: {e}"
                );
                Some(client_addr)
            } else {
                None
            }
        })
        .collect()
        .await;

        // I could use stream::iter and run all removes concurrently, but since the remove_client locks the clients, it will end up running sequentially anyway
        for addr in clients_to_remove {
            remove_client(&clients, &addr).await;
        }
    }
}

async fn remove_client(
    clients: &Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>>,
    ip_addr: &SocketAddr,
) {
    tracing::info!("Removing client from list {ip_addr}");
    clients.lock().await.remove(ip_addr);
}
