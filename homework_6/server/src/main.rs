mod args;

use args::Args;
use clap::Parser;
use shared::message::Message;
use shared::tracing::{get_subscriber, init_subscriber};
use std::{
    collections::HashMap,
    error::Error,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

fn main() {
    let args = Args::parse();

    // Setup tracing, default output is stdout.
    let tracing_subscriber = get_subscriber("server".into(), "debug".into(), std::io::stdout);
    init_subscriber(tracing_subscriber);

    if let Err(e) = start(args) {
        tracing::error!("Error while running server: {e}");
    }
}

/// Starts the server. It will listen for incoming connections and spawn a new thread for each connection.
/// In a separate thread runs a broadcasting function that will send messages to all connected clients.
fn start(args: Args) -> Result<(), Box<dyn Error>> {
    let server = format!("{}:{}", args.host, args.port);
    tracing::info!("Starting server on address {server}...");

    let listener = TcpListener::bind(server)?;

    listener
        .set_nonblocking(true)
        .expect("Cannot set non-blocking");

    let (sender, receiver) = std::sync::mpsc::channel();

    let clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));

    let broadcast_handle = thread::spawn({
        let clients = clients.clone();
        || broadcast_messages(clients, receiver)
    });

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                thread::spawn({
                    let sender = sender.clone();
                    let clients = clients.clone();
                    move || handle_connection(s, sender, clients)
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => tracing::error!("encountered IO error: {e}"),
        }
    }

    _ = broadcast_handle.join();

    Ok(())
}

/// Handles a connection from a client.
/// In a loop it will listen for incoming messages and send them to the broadcasting thread using chanel.
fn handle_connection(
    mut stream: TcpStream,
    sender: Sender<(SocketAddr, Message)>,
    clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
) -> Result<(), Box<dyn Error + Send + 'static>> {
    let addr = stream.peer_addr().unwrap();
    clients
        .lock()
        .unwrap()
        .insert(addr, stream.try_clone().unwrap());

    tracing::info!("New connection from: {addr}");

    let clients_count = clients.lock().unwrap().len();
    Message::send_active_users_msg(&mut stream, clients_count).unwrap_or_else(|e| {
        tracing::error!(e, "Unable to send message to a new connection");
    });

    while let Ok(message) = Message::receive_msg(&mut stream) {
        tracing::info!("New message from: {addr}");
        if let Err(e) = sender.send((addr, message)) {
            // If the channel is closed it means that the broadcasting thread has stopped. This means that something went wrong and messages cannot be sent to clients.
            panic!(
                "Error while sending message to channel: ip {}, error: {}",
                addr, e
            );
        }
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
        let mut clients_iter = clients.lock().unwrap();

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
    clients.lock().unwrap().remove(ip_addr);
}
