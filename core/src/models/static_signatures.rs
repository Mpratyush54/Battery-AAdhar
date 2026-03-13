use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StaticSignatures {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub data_section: String,
    pub data_hash: String,
    pub signature: Vec<u8>,
    pub certificate_id: uuid::Uuid,
    pub signed_at: chrono::NaiveDateTime,
}
