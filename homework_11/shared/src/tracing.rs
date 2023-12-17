use chrono::Utc;
use std::{
    fs::{self, File},
    path::PathBuf,
};
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

use crate::errors::TracingErrors;

//This is probably an overkill at this point but I really like the tracing library and I used it in my previous projects.

/// Returns a tracing subscriber that writes to the given `sink`. The subscriber will only trace events with the given `env_filter`.
/// name: name of the subscriber (client, server... etc.)
/// env_filter: filter for the subscriber (debug, info, error)
/// sink: where to write the logs (stdout, file)
pub fn get_subscriber<T>(name: String, env_filter: String, sink: T) -> impl Subscriber + Sync + Send
where
    T: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    //filtering logs based on severity
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    //log formatter
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer) //tracks more info for layers below (context...)
        .with(formatting_layer)
}

/// Initializes the global log subscriber with the given `subscriber`.
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) -> Result<(), TracingErrors> {
    LogTracer::init()
        .map_err(|_| TracingErrors::SetupTracingError("Failed to set logger".to_string()))?; // this is to log trace events by our application
    set_global_default(subscriber)
        .map_err(|_| TracingErrors::SetupTracingError("Failed to set subscriber".to_string()))?;
    Ok(())
}

/// Creates a log file in the `logs_dir` directory with the name `file_prefix-<timestamp>.log`
/// If the directory does not exist it will be created.
/// Returns the file to write to.
pub fn create_log_file(logs_dir: &str, file_prefix: &str) -> Result<File, TracingErrors> {
    let logs_dir = PathBuf::from(logs_dir);

    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir).map_err(TracingErrors::CreateDirError)?;
    }

    let timestamp = Utc::now().timestamp();

    let path = logs_dir.join(format!("{}_{}.log", file_prefix, timestamp));

    let file = File::create(path).map_err(TracingErrors::CreateLogFileError)?;
    Ok(file)
}
