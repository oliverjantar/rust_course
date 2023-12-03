mod configuration;
mod server_error;

use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use configuration::{get_configuration, DatabaseSettings, Settings};
use flume::{Receiver, Sender};
use futures::stream::{self, StreamExt};
use rand::{SecureRandom, SystemRandom};
use ring::{digest, pbkdf2, rand};
use secrecy::{ExposeSecret, Secret};
use server_error::ServerError;
use shared::message::{AuthPayload, Message, MessagePayload};
use shared::tracing::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use std::num::NonZeroU32;
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
                let pool = Arc::clone(&pool);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, address, sender, clients, pool).await
                    {
                        tracing::error!("Error while handling connection: {}", e);
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
    mut stream: TcpStream,
    address: SocketAddr,
    sender: Sender<(SocketAddr, Message)>,
    clients: Arc<Mutex<HashMap<SocketAddr, OwnedWriteHalf>>>,
    db_pool: Arc<Pool<Postgres>>,
) -> Result<(), ServerError> {
    tracing::info!("New connection from: {address}. Authenticating...");
    let (current_user_id, current_user_name) = authenticate_user(&mut stream, &db_pool).await?;

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

        insert_message(&db_pool, &message, &current_user_id)
            .await
            .map_err(ServerError::StoreMessage)?;

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

fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
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

const CREDENTIAL_LEN: usize = digest::SHA512_OUTPUT_LEN;
const N_ITER: Option<NonZeroU32> = NonZeroU32::new(100_000);

async fn authenticate_user(
    mut stream: &mut TcpStream,
    db_pool: &PgPool,
) -> Result<(Uuid, String), ServerError> {
    loop {
        // wait for username and password from client
        let msg = match Message::receive_msg(stream).await {
            Ok(msg) => msg,
            Err(_) => continue,
        };

        if let MessagePayload::Login(user) = msg.data {
            tracing::debug!("Received log in message from user.");
            match get_user(db_pool, &user.name).await {
                Ok(user_db) => {
                    let decoded_salt = general_purpose::STANDARD
                        .decode(user_db.salt.as_bytes())
                        .map_err(|_| ServerError::PasswordDecode)?;

                    let decoded_pwd = general_purpose::STANDARD
                        .decode(user_db.password.expose_secret().as_bytes())
                        .map_err(|_| ServerError::PasswordDecode)?;

                    if verify_pwd(&decoded_pwd, &decoded_salt, user.password.as_bytes()) {
                        tracing::debug!("Log in was successful.");

                        let payload = MessagePayload::LoginResponse(AuthPayload::new_login());

                        let msg = Message::new(payload);
                        Message::send_msg(&msg, stream)
                            .await
                            .map_err(ServerError::SendMessage)?;

                        return Ok((user_db.id, user_db.username));
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
                Err(err) => match err {
                    // If user does not exist, create one
                    sqlx::Error::RowNotFound => {
                        tracing::debug!("Registering new user.");

                        let user_id = Uuid::new_v4();
                        let user_name = user.name;

                        // hash password
                        let mut salt = [0u8; CREDENTIAL_LEN];
                        let rng = SystemRandom::new();

                        rng.fill(&mut salt).unwrap();

                        let mut pwd_hash = [0u8; CREDENTIAL_LEN];
                        pbkdf2::derive(
                            pbkdf2::PBKDF2_HMAC_SHA512,
                            N_ITER.unwrap(),
                            &salt,
                            user.password.as_bytes(),
                            &mut pwd_hash,
                        );

                        let encoded_pwd = general_purpose::STANDARD.encode(pwd_hash);
                        let encoded_salt = general_purpose::STANDARD.encode(salt);

                        insert_user(db_pool, user_id, &encoded_pwd, &user_name, &encoded_salt)
                            .await
                            .map_err(ServerError::StoreUser)?;
                        let payload = MessagePayload::LoginResponse(AuthPayload::new_register());

                        let msg = Message::new(payload);
                        Message::send_msg(&msg, &mut stream)
                            .await
                            .map_err(ServerError::SendMessage)?;

                        return Ok((user_id, user_name));
                    }
                    _ => {
                        tracing::error!("Error while storing user to database. {err}");
                        continue;
                    }
                },
            }
        }
    }
}

fn verify_pwd(secret: &[u8], salt: &[u8], password_to_verify: &[u8]) -> bool {
    pbkdf2::verify(
        pbkdf2::PBKDF2_HMAC_SHA512,
        N_ITER.unwrap(),
        salt,
        password_to_verify,
        secret,
    )
    .is_ok()
}
