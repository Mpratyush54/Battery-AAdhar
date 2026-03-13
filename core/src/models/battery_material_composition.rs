use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BatteryMaterialComposition {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub cathode_material: String,
    pub anode_material: String,
    pub electrolyte_type: String,
    pub separator_material: String,
    pub lithium_content_g: f64,
    pub cobalt_content_g: f64,
    pub nickel_content_g: f64,
    pub recyclable_percentage: f64,
    pub encrypted_details: String,
    pub created_at: chrono::NaiveDateTime,
}
