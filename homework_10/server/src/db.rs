use crate::{configuration::DatabaseSettings, server_error::ServerError, user::User};
use async_trait::async_trait;
use chrono::Utc;
use secrecy::ExposeSecret;
use shared::message::{Message, MessagePayload};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

#[async_trait]
pub trait ChatDb {
    async fn insert_message(&self, message: &Message, user_id: &Uuid) -> Result<(), ServerError>;
    async fn insert_user(&self, user: &User) -> Result<(), ServerError>;
    async fn get_user(&self, username: &str) -> Result<Option<User>, ServerError>;
}

pub struct ChatPostgresDb {
    db_pool: PgPool,
}

impl ChatPostgresDb {
    pub fn new(configuration: &DatabaseSettings) -> Self {
        let db_pool = Self::get_connection_pool(configuration);
        Self { db_pool }
    }

    fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy_with(configuration.with_db())
    }
}

#[async_trait]
impl ChatDb for ChatPostgresDb {
    #[tracing::instrument(skip(self, message))]
    async fn insert_message(&self, message: &Message, user_id: &Uuid) -> Result<(), ServerError> {
        let data = MessagePayload::serialize_to_text(&message.data);
        sqlx::query!(
            r#"
            INSERT INTO messages(id,user_id,data,timestamp)
            VALUES ($1,$2,$3,$4)
            "#,
            Uuid::new_v4(),
            user_id,
            &data,
            Utc::now(),
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            ServerError::StoreMessage
        })?;
        Ok(())
    }

    #[tracing::instrument(skip(self, user))]
    async fn insert_user(&self, user: &User) -> Result<(), ServerError> {
        sqlx::query!(
            r#"
            INSERT INTO users(id,password,username,salt,last_login)
            VALUES ($1,$2,$3,$4,$5)
            "#,
            user.id,
            user.password.expose_secret(),
            user.username,
            user.salt,
            Utc::now(),
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            ServerError::StoreUser
        })?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn get_user(&self, username: &str) -> Result<Option<User>, ServerError> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, password, username, salt FROM users WHERE username = $1",
            username
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            ServerError::GetUser
        })?;

        Ok(user)
    }
}
