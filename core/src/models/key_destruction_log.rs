use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KeyDestructionLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub dek_version: i32,
    pub destroyed_by: uuid::Uuid,
    pub destruction_method: String,
    pub destroyed_at: chrono::NaiveDateTime,
}
