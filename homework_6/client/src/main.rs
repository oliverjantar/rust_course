mod args;
mod client;
mod command;
mod utils;

use args::Args;
use clap::Parser;
use client::Client;
use shared::tracing::{create_log_file, get_subscriber, init_subscriber};
use std::io::Write;
use utils::log_error;

use std::{error::Error, thread};

fn main() {
    let args = Args::parse();
    setup_tracing(&args.logs_dir);

    let output_writer = std::io::stdout();

    if let Err(e) = start(args, output_writer) {
        log_error(e);
    }
}

fn setup_tracing(logs_dir: &str) {
    let log_file = create_log_file(logs_dir, "client").expect("Failed to create log file");

    let tracing_subscriber = get_subscriber("client".into(), "debug".into(), log_file); //std::io::stdout);
    init_subscriber(tracing_subscriber);
}

#[tracing::instrument(name = "Starting client", skip(writer))]
fn start<T>(args: Args, writer: T) -> Result<(), Box<dyn Error>>
where
    T: Write + Send + 'static,
{
    let (client_sender, client_receiver) =
        Client::connect(args.host, args.port, writer, &args.output_dir)?;

    let _ = thread::spawn(|| client_receiver.start());

    client_sender.start()?;

    Ok(())
}
