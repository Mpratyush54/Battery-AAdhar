use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KeyRotationLog {
    pub id: uuid::Uuid,
    pub key_type: String,
    pub previous_version: i32,
    pub new_version: i32,
    pub initiated_by: uuid::Uuid,
    pub approved_by: uuid::Uuid,
    pub approval_timestamp: chrono::NaiveDateTime,
    pub rotated_at: chrono::NaiveDateTime,
}
