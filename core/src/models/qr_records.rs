use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct QrRecords {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub qr_payload_hash: String,
    pub generated_at: chrono::NaiveDateTime,
}
