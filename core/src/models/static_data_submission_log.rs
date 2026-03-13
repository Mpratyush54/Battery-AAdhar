use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StaticDataSubmissionLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub submitted_by: uuid::Uuid,
    pub data_section: String,
    pub data_hash: String,
    pub previous_event_hash: String,
    pub event_hash: String,
    pub submitted_at: chrono::NaiveDateTime,
}
