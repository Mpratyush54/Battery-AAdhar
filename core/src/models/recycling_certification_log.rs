use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecyclingCertificationLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub previous_event_hash: String,
    pub event_hash: String,
    pub recycler_hash: String,
    pub material_recovery_hash: String,
    pub certified_at: chrono::NaiveDateTime,
}
