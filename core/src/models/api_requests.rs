use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiRequests {
    pub id: uuid::Uuid,
    pub parent_request_id: uuid::Uuid,
    pub request_hash: String,
    pub endpoint_hash: String,
    pub subject_hash: String,
    pub status_hash: String,
    pub latency_ms: i32,
    pub created_at: chrono::NaiveDateTime,
}
