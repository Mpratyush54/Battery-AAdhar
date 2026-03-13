use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BatteryKeys {
    pub bpan: String,
    pub encrypted_dek: Vec<u8>,
    pub kek_version: i32,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub key_status: String,
    pub created_at: chrono::NaiveDateTime,
    pub rotated_at: chrono::NaiveDateTime,
}
