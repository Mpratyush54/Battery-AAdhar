use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DataAccessExecutionLog {
    pub id: uuid::Uuid,
    pub stakeholder_id: uuid::Uuid,
    pub bpan: String,
    pub resource_type: String,
    pub access_type: String,
    pub granted: bool,
    pub accessed_at: chrono::NaiveDateTime,
}
