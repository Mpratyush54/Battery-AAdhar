use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Manufacturers {
    pub id: uuid::Uuid,
    pub manufacturer_code: String,
    pub name: String,
    pub country_code: String,
    pub encrypted_profile: String,
    pub created_at: chrono::NaiveDateTime,
}
