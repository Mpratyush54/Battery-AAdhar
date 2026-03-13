use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OwnershipHistory {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub encrypted_owner_identity: String,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
}
