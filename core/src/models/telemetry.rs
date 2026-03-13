use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Telemetry {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub encrypted_payload: String,
    pub recorded_at: chrono::NaiveDateTime,
}
