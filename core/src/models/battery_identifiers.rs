use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BatteryIdentifiers {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub encrypted_serial_number: String,
    pub encrypted_batch_number: String,
    pub encrypted_factory_code: String,
    pub created_at: chrono::NaiveDateTime,
}
