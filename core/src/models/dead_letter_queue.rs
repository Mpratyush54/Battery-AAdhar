use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeadLetterQueue {
    pub id: uuid::Uuid,
    pub original_message_id: uuid::Uuid,
    pub failure_reason_hash: String,
    pub retry_count: i32,
    pub created_at: chrono::NaiveDateTime,
}
