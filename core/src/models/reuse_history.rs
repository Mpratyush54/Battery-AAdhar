use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReuseHistory {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub reuse_application: String,
    pub certified_by: String,
    pub certified_at: chrono::NaiveDateTime,
}
