use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Stakeholders {
    pub id: uuid::Uuid,
    pub role: String,
    pub encrypted_profile: String,
    pub created_at: chrono::NaiveDateTime,
}
