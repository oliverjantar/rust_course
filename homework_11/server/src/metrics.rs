use lazy_static::lazy_static;
use prometheus::{Gauge, IntCounter, Opts};

lazy_static! {
    pub static ref MESSAGES_COUNTER: IntCounter = IntCounter::new(
        "messages_counter",
        "How many messages were sent to clients. Including server info messages."
    )
    .unwrap();
    pub static ref ACTIVE_CONNECTIONS: Gauge = {
        let gauge_opts = Opts::new(
            "active_connections_counter",
            "How many clients are connected to the server.",
        );
        Gauge::with_opts(gauge_opts).expect("Failed to create gauge")
    };
}

pub fn register_metrics() {
    prometheus::default_registry()
        .register(Box::new(MESSAGES_COUNTER.clone()))
        .expect("Failed to register message counter");

    prometheus::default_registry()
        .register(Box::new(ACTIVE_CONNECTIONS.clone()))
        .expect("Failed to register connections counter");
}
