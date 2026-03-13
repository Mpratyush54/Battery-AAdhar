use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct JobExecutionLog {
    pub id: uuid::Uuid,
    pub job_id: uuid::Uuid,
    pub status: String,
    pub duration_ms: i32,
    pub executed_at: chrono::NaiveDateTime,
}
