mod args;
use args::Args;
use clap::Parser;
use shared::message_type::{receive_msg, send_msg, MessageType};
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

    if let Err(e) = start(args) {
        eprintln!("Error while running server: {e}");
    }
}

fn start(args: Args) -> Result<(), Box<dyn Error>> {
    let server = format!("{}:{}", args.host, args.port);
    println!("Starting server on address {server}...");

    let listener = TcpListener::bind(server)?;

    listener
        .set_nonblocking(true)
        .expect("Cannot set non-blocking");

    let (sender, receiver) = std::sync::mpsc::channel();
    let sender = Arc::new(sender);

    let clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));

    let broadcast_handle = thread::spawn({
        let clients = clients.clone();
        || broadcast_messages(clients, receiver)
    });

    //todo change to iterator
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
            Err(e) => eprintln!("encountered IO error: {e}"),
        }
    }

    _ = broadcast_handle.join();

    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    sender: Arc<Sender<(SocketAddr, MessageType)>>,
    clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
) -> Result<(), Box<dyn Error + Send + 'static>> {
    let addr = stream.peer_addr().unwrap();
    clients
        .lock()
        .unwrap()
        .insert(addr, stream.try_clone().unwrap());

    println!("New connection from: {addr}");

    while let Ok(message) = receive_msg(&mut stream) {
        println!("New message from: {addr}");
        if let Err(e) = sender.send((addr, message)) {
            panic!(
                "Error while sending message to channel: ip {}, error: {}",
                addr, e
            );
        }
    }

    remove_client(&clients, &addr);
    Ok(())
}

fn broadcast_messages(
    clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
    receiver: Receiver<(SocketAddr, MessageType)>,
) {
    while let Ok((ref ip_addr, message)) = receiver.recv() {
        let mut clients_to_remove = vec![];
        let mut clients_iter = clients.lock().unwrap();
        //todo itertools filter
        for (client_addr, stream) in clients_iter.iter_mut() {
            if ip_addr != client_addr {
                if let Err(e) = send_msg(&message, stream) {
                    eprintln!(
                        "Error while broadcasting message to client {client_addr}. Error: {e}",
                    );
                    clients_to_remove.push(client_addr);
                }
            }
        }

        clients_to_remove.iter().for_each(|&addr| {
            remove_client(&clients, addr);
        });
    }
}

fn remove_client(clients: &Arc<Mutex<HashMap<SocketAddr, TcpStream>>>, ip_addr: &SocketAddr) {
    println!("Removing client from list {ip_addr}");
    clients.lock().unwrap().remove(ip_addr);
}
