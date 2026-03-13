use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IdempotencyKeys {
    pub id: uuid::Uuid,
    pub request_hash: String,
    pub response_hash: String,
    pub expires_at: chrono::NaiveDateTime,
}
