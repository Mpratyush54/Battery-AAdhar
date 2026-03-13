use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReuseCertificationLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub previous_event_hash: String,
    pub event_hash: String,
    pub application_type: String,
    pub certifier_hash: String,
    pub certified_at: chrono::NaiveDateTime,
}
