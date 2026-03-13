use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DynamicDataLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub previous_event_hash: String,
    pub event_hash: String,
    pub upload_type: String,
    pub record_hash: String,
    pub uploaded_at: chrono::NaiveDateTime,
}
