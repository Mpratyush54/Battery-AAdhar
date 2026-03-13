use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BatteryRegistrationLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub manufacturer_id: uuid::Uuid,
    pub registration_status: String,
    pub submitted_at: chrono::NaiveDateTime,
    pub approved_at: chrono::NaiveDateTime,
    pub approved_by: uuid::Uuid,
}
