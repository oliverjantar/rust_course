mod args;
mod configuration;
mod server_error;

use anyhow::Result;
use chrono::Utc;
use configuration::{get_configuration, DatabaseSettings, Settings};
use flume::{Receiver, Sender};
use futures::stream::{self, StreamExt};
use secrecy::{ExposeSecret, Secret};
use server_error::ServerError;
use shared::message::{AuthPayload, Message, MessagePayload};
use shared::tracing::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // Setup tracing, default output is stdout.
    let tracing_subscriber = get_subscriber("server".into(), "debug".into(), std::io::stdout);
    if let Err(e) = init_subscriber(tracing_subscriber) {
        tracing::error!("Error while setting up server. {e}");
        return;
    }

    let configuration = get_configuration().expect("Failed to read configuration.");

    if let Err(e) = start(configuration).await {
        tracing::error!("Error while running server. {e}");
    }
}

/// Starts the server. It will listen for incoming connections and spawn a new thread for each connection.
/// In a separate thread runs a broadcasting function that will send messages to all connected clients.
async fn start(config: Settings) -> Result<()> {
    let connection_pool = get_connection_pool(&config.database);

    let pool = Arc::new(connection_pool);

    // let port = listener.local_addr().unwrap().port();

    let server = format!("{}:{}", config.application.host, config.application.port);
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
                let clients = Arc::clone(&clients);
                let pool = Arc::clone(&pool);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, address, sender, clients, pool).await
                    {
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
    db_pool: Arc<Pool<Postgres>>,
) -> Result<(), ServerError> {
    let (mut read_half, mut write_half) = stream.into_split();

    tracing::info!("New connection from: {address}. Authenticating...");
    let current_user_id: Uuid;
    let current_user_name: String;

    loop {
        let msg = match Message::receive_msg(&mut read_half).await {
            Ok(msg) => msg,
            Err(_) => continue,
        };

        if let MessagePayload::Login(user) = msg.data {
            tracing::debug!("Received log in message from user.");
            match get_user(&db_pool, &user.name).await {
                Ok(user_db) => {
                    if user_db.password.expose_secret() == &user.password {
                        tracing::debug!("Log in was successful.");
                        current_user_id = user_db.id;
                        current_user_name = user_db.username;
                        let payload = MessagePayload::LoginResponse(AuthPayload::new_login());

                        let msg = Message::new(payload);
                        Message::send_msg(&msg, &mut write_half)
                            .await
                            .map_err(ServerError::SendMessage)?;

                        break;
                    } else {
                        tracing::debug!("Incorrect password.");

                        let payload = MessagePayload::LoginResponse(AuthPayload::new_error());

                        let msg = Message::new(payload);
                        Message::send_msg(&msg, &mut write_half)
                            .await
                            .map_err(ServerError::SendMessage)?;
                        continue;
                    }
                }
                Err(err) => match err {
                    sqlx::Error::RowNotFound => {
                        tracing::debug!("Registering new user.");
                        current_user_id = Uuid::new_v4();
                        current_user_name = user.name;

                        insert_user(
                            &db_pool,
                            current_user_id,
                            &user.password,
                            &current_user_name,
                            "salt",
                        )
                        .await
                        .map_err(ServerError::StoreUser)?;
                        let payload = MessagePayload::LoginResponse(AuthPayload::new_register());

                        let msg = Message::new(payload);
                        Message::send_msg(&msg, &mut write_half)
                            .await
                            .map_err(ServerError::SendMessage)?;
                        break;
                    }
                    _ => {
                        tracing::error!("Error while storing user to database. {err}");
                        continue;
                    }
                },
            }
        }
    }

    let clients_count = clients.lock().await.len();

    Message::send_active_users_msg(&mut write_half, clients_count)
        .await
        .map_err(ServerError::SendMessage)?;

    clients.lock().await.insert(address, write_half);

    // Broadcast to other users that new user was connected
    let msg = Message::new_server_msg(&format!("New user connected: {}", current_user_name));
    sender
        .send((address, msg))
        .map_err(ServerError::ChannelSend)?;

    // Start receiving messages from user and broadcast them
    while let Ok(mut message) = Message::receive_msg(&mut read_half).await {
        tracing::info!("New message from: {address}");

        insert_message(&db_pool, &message, &current_user_id)
            .await
            .map_err(ServerError::StoreMessage)?;

        message.set_sender(&current_user_name);

        sender
            .send((address, message))
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

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

#[tracing::instrument(skip(db_pool, message))]
async fn insert_message(
    db_pool: &PgPool,
    message: &Message,
    user_id: &Uuid,
) -> Result<(), sqlx::Error> {
    let data = shared::message::MessagePayload::serialize_to_text(&message.data);
    sqlx::query!(
        r#"
        INSERT INTO messages(id,user_id,data,timestamp)
        VALUES ($1,$2,$3,$4)
        "#,
        Uuid::new_v4(),
        user_id,
        &data,
        Utc::now(),
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(skip(db_pool, pwd, salt))]
async fn insert_user(
    db_pool: &PgPool,
    user_id: Uuid,
    pwd: &str,
    username: &str,
    salt: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO users(id,password,username,salt,last_login)
        VALUES ($1,$2,$3,$4,$5)
        "#,
        user_id,
        pwd,
        username,
        salt,
        Utc::now(),
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(skip(db_pool))]
async fn get_user(db_pool: &PgPool, username: &str) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        "SELECT id, password, username, salt FROM users WHERE username = $1",
        username
    )
    .fetch_one(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(user)
}

#[derive(Debug)]
struct User {
    id: Uuid,
    password: Secret<String>,
    username: String,
    salt: String,
}
