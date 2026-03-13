use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KekKeys {
    pub id: uuid::Uuid,
    pub encrypted_kek: Vec<u8>,
    pub version: i32,
    pub root_key_id: uuid::Uuid,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
    pub retired_at: chrono::NaiveDateTime,
}
