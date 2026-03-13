use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduledJobs {
    pub id: uuid::Uuid,
    pub job_name_hash: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub last_run: chrono::NaiveDateTime,
}
