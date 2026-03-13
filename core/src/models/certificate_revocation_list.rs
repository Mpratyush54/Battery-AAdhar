use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CertificateRevocationList {
    pub id: uuid::Uuid,
    pub certificate_id: uuid::Uuid,
    pub revoked_by_hash: String,
    pub reason: String,
    pub revoked_at: chrono::NaiveDateTime,
}
