use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ComplianceViolationLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub violation_type: String,
    pub severity: String,
    pub detected_at: chrono::NaiveDateTime,
    pub resolved: bool,
}
