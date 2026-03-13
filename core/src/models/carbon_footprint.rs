use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CarbonFootprint {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub raw_material_emission: f64,
    pub manufacturing_emission: f64,
    pub transport_emission: f64,
    pub usage_emission: f64,
    pub recycling_emission: f64,
    pub total_emission: f64,
    pub verified: bool,
    pub created_at: chrono::NaiveDateTime,
}
