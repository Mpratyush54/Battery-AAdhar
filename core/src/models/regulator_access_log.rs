use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RegulatorAccessLog {
    pub id: uuid::Uuid,
    pub stakeholder_id: uuid::Uuid,
    pub bpan: String,
    pub reason: String,
    pub accessed_at: chrono::NaiveDateTime,
}
