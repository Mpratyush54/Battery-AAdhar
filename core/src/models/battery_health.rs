use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BatteryHealth {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub state_of_health: f64,
    pub total_cycles: i32,
    pub degradation_class: String,
    pub end_of_life: bool,
    pub updated_at: chrono::NaiveDateTime,
}
