use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RateLimits {
    pub id: uuid::Uuid,
    pub subject_hash: String,
    pub window_start: chrono::NaiveDateTime,
    pub request_count: i32,
}
