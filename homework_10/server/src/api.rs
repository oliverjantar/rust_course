use actix_web::{dev::Server, web, App, HttpServer};
use actix_web::{HttpResponse, Responder};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::{
    configuration::Settings,
    db::{ChatDb, ChatPostgresDb},
};

pub struct Api {
    port: u16,
    server: Server,
}

impl Api {
    pub fn build(config: Settings) -> Result<Self, std::io::Error> {
        let db = ChatPostgresDb::new(&config.database);

        let address = format!(
            "{}:{}",
            config.application.host, config.application.api_port
        );
        tracing::info!("Starting api on address {address}...");

        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, db)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

fn run<T>(listener: std::net::TcpListener, db_pool: T) -> Result<Server, std::io::Error>
where
    T: ChatDb + Send + Sync + 'static,
{
    let db_pool = web::Data::new(db_pool);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health", web::get().to(health_check))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}
