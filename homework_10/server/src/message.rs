use chrono::{DateTime, Utc};
use serde::Serialize;

// #[derive(Serialize)]
pub struct MessageInfo {
    pub username: String,
    pub text: String,
    pub timestamp: DateTime<Utc>,
}
