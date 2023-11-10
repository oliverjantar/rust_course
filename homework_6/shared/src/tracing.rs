use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber<T>(name: String, env_filter: String, sink: T) -> impl Subscriber + Sync + Send
where
    T: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter)); //filtering logs based on severity

    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    //let formatting_layer = BunyanFormattingLayer::new(name, std::io::stdout); //log formatter

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer) //tracks more info for layers below (request_id, context...)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger"); // this is to log trace events by our application
    set_global_default(subscriber).expect("Failed to set subscriber");
}