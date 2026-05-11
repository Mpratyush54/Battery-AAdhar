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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialComposition {
    pub bpan: String,
    pub cell_type: String,
    pub chemistry_type: String,
    pub cathode_material: String,
    pub anode_material: String,
    pub electrolyte_type: String,
    pub separator_type: String,
    pub bms_type: String,
    pub bms_version: String,
    pub cooling_system: Option<String>,
    pub heating_system: Option<String>,
    pub terminal_type: String,
    pub case_material: String,
    pub weight_kg: f32,
    pub dimensions: String,
    pub internal_resistance_mohm: f32,
    pub nominal_capacity_ah: f32,
    pub warranty_years: u8,
    pub cycle_life_80_percent: u32,
    pub operating_temp_range: String,
    pub environmental_compliance: String,
    pub recyclable_percentage: f32,
    pub recycling_instructions: Option<String>,
    pub submitted_by: String,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
}

impl MaterialComposition {
    pub fn from_request(bpan: String, req: MaterialCompositionRequest, submitted_by: String) -> Self {
        Self {
            bpan,
            cell_type: req.cell_type,
            chemistry_type: req.chemistry_type,
            cathode_material: req.cathode_material,
            anode_material: req.anode_material,
            electrolyte_type: req.electrolyte_type,
            separator_type: req.separator_type,
            bms_type: req.bms_type,
            bms_version: req.bms_version,
            cooling_system: req.cooling_system,
            heating_system: req.heating_system,
            terminal_type: req.terminal_type,
            case_material: req.case_material,
            weight_kg: req.weight_kg,
            dimensions: req.dimensions,
            internal_resistance_mohm: req.internal_resistance_mohm,
            nominal_capacity_ah: req.nominal_capacity_ah,
            warranty_years: req.warranty_years,
            cycle_life_80_percent: req.cycle_life_80_percent,
            operating_temp_range: req.operating_temp_range,
            environmental_compliance: req.environmental_compliance,
            recyclable_percentage: req.recyclable_percentage,
            recycling_instructions: req.recycling_instructions,
            submitted_by,
            submitted_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCompositionRequest {
    pub cell_type: String,
    pub chemistry_type: String,
    pub cathode_material: String,
    pub anode_material: String,
    pub electrolyte_type: String,
    pub separator_type: String,
    pub bms_type: String,
    pub bms_version: String,
    pub cooling_system: Option<String>,
    pub heating_system: Option<String>,
    pub terminal_type: String,
    pub case_material: String,
    pub weight_kg: f32,
    pub dimensions: String,
    pub internal_resistance_mohm: f32,
    pub nominal_capacity_ah: f32,
    pub warranty_years: u8,
    pub cycle_life_80_percent: u32,
    pub operating_temp_range: String,
    pub environmental_compliance: String,
    pub recyclable_percentage: f32,
    pub recycling_instructions: Option<String>,
}
