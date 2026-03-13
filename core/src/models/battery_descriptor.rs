use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BatteryDescriptor {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub chemistry_type: String,
    pub nominal_voltage: f64,
    pub rated_capacity_kwh: f64,
    pub energy_density: f64,
    pub weight_kg: f64,
    pub form_factor: String,
    pub created_at: chrono::NaiveDateTime,
}
