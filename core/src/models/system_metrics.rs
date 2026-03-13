use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemMetrics {
    pub id: uuid::Uuid,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub metric_name_hash: String,
    pub metric_value_cipher: String,
    pub recorded_at: chrono::NaiveDateTime,
}
