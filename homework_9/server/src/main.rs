use server::configuration::get_configuration;
use server::startup::start;
use shared::tracing::{get_subscriber, init_subscriber};

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
