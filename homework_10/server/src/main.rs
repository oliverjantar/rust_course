use std::fmt::{Debug, Display};

use server::startup::start;
use server::{api::Api, configuration::get_configuration};
use shared::tracing::{get_subscriber, init_subscriber};
use tokio::task::JoinError;

#[tokio::main]
async fn main() {
    // Setup tracing, default output is stdout.
    let tracing_subscriber = get_subscriber("server".into(), "debug".into(), std::io::stdout);
    if let Err(e) = init_subscriber(tracing_subscriber) {
        tracing::error!("Error while setting up server. {e}");
        return;
    }

    let configuration = get_configuration().expect("Failed to read configuration.");

    let Ok(api) = Api::build(configuration.clone()) else {
        tracing::error!("Error while setting up api.");
        return;
    };

    let api_task = tokio::spawn(api.run_until_stopped());
    let chat_server_task = tokio::spawn(start(configuration));

    tokio::select! {
        o = chat_server_task => log_exit("Chat server", o),
        o = api_task => log_exit("Api", o)
    };
}

fn log_exit(name: &str, result: Result<Result<(), impl Debug + Display>, JoinError>) {
    match result {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", name)
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{}' task failed to complete",
                name
            )
        }
    }
}
