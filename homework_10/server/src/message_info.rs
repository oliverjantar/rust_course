use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct MessageInfo {
    pub id: Uuid,
    pub username: String,
    pub text: String,
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
}
