use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Alerts {
    pub id: uuid::Uuid,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub severity_hash: String,
    pub message_cipher: String,
    pub triggered_at: chrono::NaiveDateTime,
    pub resolved: bool,
}
