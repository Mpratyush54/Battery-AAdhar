use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Batteries {
    pub bpan: String,
    pub manufacturer_id: uuid::Uuid,
    pub production_year: i32,
    pub battery_category: String,
    pub compliance_class: String,
    pub static_hash: String,
    pub carbon_hash: String,
    pub created_at: chrono::NaiveDateTime,
}
