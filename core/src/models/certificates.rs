use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Certificates {
    pub id: uuid::Uuid,
    pub public_key: String,
    pub issued_by_hash: String,
    pub issued_at: chrono::NaiveDateTime,
    pub expires_at: chrono::NaiveDateTime,
    pub revoked: bool,
}
