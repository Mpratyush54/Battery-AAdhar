use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RootKeys {
    pub id: uuid::Uuid,
    pub key_identifier: String,
    pub hardware_backed: bool,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
    pub retired_at: chrono::NaiveDateTime,
}
