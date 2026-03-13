use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemIntegrityLog {
    pub id: uuid::Uuid,
    pub check_type: String,
    pub status: String,
    pub checked_at: chrono::NaiveDateTime,
}
