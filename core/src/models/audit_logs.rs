use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogs {
    pub id: uuid::Uuid,
    pub actor_id: uuid::Uuid,
    pub action: String,
    pub resource: String,
    pub previous_hash: String,
    pub entry_hash: String,
    pub created_at: chrono::NaiveDateTime,
}
