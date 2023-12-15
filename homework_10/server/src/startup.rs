use configuration::Settings;
use flume::{Receiver, Sender};
use futures::stream::{self, StreamExt};
use server_error::ServerError;
use shared::message::{AuthPayload, Message, MessagePayload};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db::{ChatDb, ChatPostgresDb};
use crate::{configuration, server_error};

/// Starts the server. It will listen for incoming connections and spawn a new thread for each connection.
/// In a separate thread runs a broadcasting function that will send messages to all connected clients.
pub async fn start(config: Settings) -> Result<(), ServerError> {
    let db = Arc::new(ChatPostgresDb::new(&config.database));

    let server = format!("{}:{}", config.application.host, config.application.port);
    tracing::info!("Starting server on address {server}...");

    let listener = TcpListener::bind(server).await.map_err(ServerError::Bind)?;

    let (sender, receiver) = flume::unbounded();

    let clients: Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>> =
        Arc::new(Mutex::new(HashMap::new()));

    tokio::spawn({
        let clients = clients.clone();
        broadcast_messages(clients, receiver)
    });

    loop {
        match listener.accept().await {
            Ok((stream, address)) => {
                let sender = sender.clone();
                let clients = Arc::clone(&clients);
                let db = Arc::clone(&db);
                tokio::spawn(async move {
                    tracing::debug!("New connection");
                    if let Err(e) = handle_connection(stream, address, sender, clients, db).await {
                        tracing::error!("Error while handling connection: {}", e);
                    }
                    tracing::debug!("Connection ended.")
                });
            }
            Err(e) => tracing::error!("Encountered network error from Tcp stream: {e}"),
        }
    }
}

/// Handles a connection from a client.
/// In a loop it will listen for incoming messages and send them to the broadcasting thread using chanel.
async fn handle_connection(
    mut stream: TcpStream,
    address: SocketAddr,
    sender: Sender<(SocketAddr, Message)>,
    clients: Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>>,
    db: Arc<impl ChatDb>,
) -> Result<(), ServerError> {
    tracing::info!("New connection from: {address}. Authenticating...");
    let (current_user_id, current_user_name) = authenticate_user(&mut stream, db.clone()).await?;

    let clients_count = clients.lock().await.len();

    let (mut read_half, mut write_half) = stream.into_split();

    Message::send_active_users_msg(&mut write_half, clients_count)
        .await
        .map_err(ServerError::SendMessage)?;

    clients.lock().await.insert(address, write_half);

    // Broadcast to other users that new user was connected
    let msg = Message::new_server_msg(&format!("New user connected: {}", current_user_name));
    sender
        .send_async((address, msg))
        .await
        .map_err(ServerError::ChannelSend)?;

    // Start receiving messages from user and broadcast them
    while let Ok(mut message) = Message::receive_msg(&mut read_half).await {
        tracing::info!("New message from: {address}");
        _ = db.insert_message(&message, &current_user_id).await;

        message.set_from_user(&current_user_name);

        sender
            .send_async((address, message))
            .await
            .map_err(ServerError::ChannelSend)?;
    }

    // If the user disconnects, we remove it from the list of connected clients.
    remove_client(&clients, &address).await;
    Ok(())
}

/// Broadcasts messages to all connected clients.
/// If a client is disconnected it will be removed from the list of connected clients.
async fn broadcast_messages(
    clients: Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>>,
    receiver: Receiver<(SocketAddr, Message)>,
) {
    let mut recv_stream = receiver.into_stream();

    while let Some((ip_addr, ref message)) = recv_stream.next().await {
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

async fn authenticate_user(
    mut stream: &mut TcpStream,
    db: Arc<impl ChatDb>,
) -> Result<(Uuid, String), ServerError> {
    loop {
        // wait for username and password from client
        let msg: Message = match Message::receive_msg(stream).await {
            Ok(msg) => msg,
            Err(err) => {
                tracing::debug!(
                    "Error while receiving msg. Message was malformed or connection ended. {}",
                    err
                );
                return Err(ServerError::ClosedConnection);
            }
        };

        if let MessagePayload::Login(auth_user) = msg.data {
            tracing::debug!("Received log in message from user.");
            if let Ok(user_db) = db.get_user(&auth_user.name).await {
                match user_db {
                    Some(user) => {
                        let verification_result =
                            user.verify_user_password(auth_user.password.as_bytes());

                        if verification_result.is_ok_and(|is_verified| is_verified) {
                            tracing::debug!("Login was successful.");

                            let payload = MessagePayload::LoginResponse(AuthPayload::new_login());

                            let msg = Message::new(payload);
                            Message::send_msg(&msg, stream)
                                .await
                                .map_err(ServerError::SendMessage)?;

                            return Ok((user.id, user.username));
                        } else {
                            tracing::debug!("Incorrect password.");

                            let payload = MessagePayload::LoginResponse(AuthPayload::new_error());

                            let msg = Message::new(payload);
                            Message::send_msg(&msg, stream)
                                .await
                                .map_err(ServerError::SendMessage)?;
                            continue;
                        }
                    }
                    // If user does not exist, create one
                    None => {
                        tracing::debug!("Registering new user.");

                        let Ok(user) = auth_user.try_into() else {
                            let payload = MessagePayload::LoginResponse(AuthPayload::new_error());

                            let msg = Message::new(payload);
                            Message::send_msg(&msg, stream)
                                .await
                                .map_err(ServerError::SendMessage)?;
                            continue;
                        };

                        if db.insert_user(&user).await.is_err() {
                            let payload = MessagePayload::LoginResponse(AuthPayload::new_error());

                            let msg = Message::new(payload);
                            Message::send_msg(&msg, stream)
                                .await
                                .map_err(ServerError::SendMessage)?;
                            continue;
                        } else {
                            let payload =
                                MessagePayload::LoginResponse(AuthPayload::new_register());

                            let msg = Message::new(payload);
                            Message::send_msg(&msg, &mut stream)
                                .await
                                .map_err(ServerError::SendMessage)?;

                            return Ok((user.id, user.username));
                        }
                    }
                }
            }
        }
    }
}
