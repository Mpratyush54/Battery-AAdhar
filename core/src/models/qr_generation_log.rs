use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct QrGenerationLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub payload_hash: String,
    pub generated_at: chrono::NaiveDateTime,
}
