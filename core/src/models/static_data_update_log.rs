use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StaticDataUpdateLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub updated_by: uuid::Uuid,
    pub field_name: String,
    pub previous_hash: String,
    pub new_hash: String,
    pub updated_at: chrono::NaiveDateTime,
}
