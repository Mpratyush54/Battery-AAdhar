use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OwnershipTransferLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub previous_event_hash: String,
    pub event_hash: String,
    pub from_owner_hash: String,
    pub to_owner_hash: String,
    pub transfer_reason: String,
    pub transferred_at: chrono::NaiveDateTime,
}
