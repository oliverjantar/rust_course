mod args;
mod client;
mod client_error;
mod command;
mod encryption;
mod utils;

use anyhow::Result;
use args::Args;
use clap::Parser;
use client::Client;
use shared::tracing::{create_log_file, get_subscriber, init_subscriber};

use std::io::Write;
use std::thread;

fn main() {
    let args = Args::parse();
    if let Err(e) = setup_tracing(&args.logs_dir) {
        let msg = "Error while starting a chat client.";
        log_error(msg, e);
        return;
    }

    let output_writer = std::io::stdout();

    if let Err(e) = start(args, output_writer) {
        let msg = "Error while running client.";
        log_error(msg, e);
    }
}

/// Sets up tracing for the client.
/// The logs will be written to the `logs_dir` directory. The default tracing file is ./logs/client-<timestamp>.log
/// I didn't want to mix up the tracing logs and chat messages so the default output is a file.
fn setup_tracing(logs_dir: &str) -> Result<()> {
    let log_file = create_log_file(logs_dir, "client")?;

    let tracing_subscriber = get_subscriber("client".into(), "debug".into(), log_file);
    init_subscriber(tracing_subscriber)?;
    Ok(())
}

/// Starts the client. It will connect to the server and start listening for commands.
/// Receiving messages will be handled in a separate thread.
#[tracing::instrument(name = "Starting client", skip(writer))]
fn start<T>(args: Args, writer: T) -> Result<()>
where
    T: Write + Send + 'static,
{
    let (client_sender, client_receiver) = Client::connect(
        writer,
        args.host,
        args.port,
        &args.output_dir,
        &args.username,
        args.enable_encryption,
        &args.encryption_key,
    )?;

    let _ = thread::spawn(|| client_receiver.start());

    client_sender.start()?;

    Ok(())
}

fn log_error(msg: &str, e: anyhow::Error) {
    tracing::error!("{msg} {e}");
    eprintln!("{msg} {e}");
}
