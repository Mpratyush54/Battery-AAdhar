use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notifications {
    pub id: uuid::Uuid,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub recipient_hash: String,
    pub encrypted_message: String,
    pub status_hash: String,
    pub created_at: chrono::NaiveDateTime,
}
