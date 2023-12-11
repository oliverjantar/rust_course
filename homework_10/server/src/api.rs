use actix_cors::Cors;
use actix_web::http::header::ContentType;
use actix_web::{dev::Server, web, App, HttpServer};
use actix_web::{HttpResponse, Responder};
use serde::Deserialize;
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

fn run(listener: std::net::TcpListener, db_pool: ChatPostgresDb) -> Result<Server, std::io::Error>
// where
//     T: ChatDb + Sync + Send,
{
    let db_pool = web::Data::new(db_pool);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .wrap(TracingLogger::default())
            .route("/health", web::get().to(health_check))
            .route("/messages", web::get().to(get_messages::<ChatPostgresDb>))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[derive(Deserialize, Debug)]
struct MessageQuery {
    username: Option<String>,
}

#[tracing::instrument(skip(db))]
async fn get_messages<T>(db: web::Data<T>, query: web::Query<MessageQuery>) -> impl Responder
where
    T: ChatDb + Sync + Send,
{
    match db
        .get_messages(query.username.as_deref().unwrap_or(""))
        .await
    {
        Ok(messages) => {
            let Ok(body) = serde_json::to_string(&messages) else {
                tracing::error!("Error while serializing messages.");
                return HttpResponse::InternalServerError().finish();
            };
            HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(body)
        }
        Err(e) => {
            tracing::error!("Error while getting messages from db. {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
