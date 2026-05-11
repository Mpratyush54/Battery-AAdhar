use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use chrono::{Utc, DateTime};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BatteryDescriptor {
    pub id: uuid::Uuid,
    pub bpan: String,

    // === Electrical ===
    pub capacity_kwh: f32, // e.g., 30 kWh
    pub nominal_voltage_v: f32, // e.g., 307 V
    pub nominal_current_a: f32, // max continuous current

    // === Chemistry & Cell ===
    pub chemistry_type: String, // NMC, LFP, NCA
    pub cell_type: String, // Cylindrical 21700, Pouch, Prismatic
    pub cell_count: i32, // Number of cells in series
    pub cell_voltage_nominal_v: f32, // Per-cell nominal voltage

    // === Manufacturing ===
    pub manufacturer_id: uuid::Uuid,
    pub manufacturing_country: String,
    pub manufacturing_facility: String,
    pub manufacture_date: String, // YYYY-MM-DD

    // === Compliance ===
    pub declared_cycle_life: i32, // Cycles to 80% SoH
    pub warranty_years: i32,

    // === Metadata ===
    pub registered_at: DateTime<Utc>,
    pub battery_hash: String, // SHA256 of all fields (immutable integrity)
}

impl BatteryDescriptor {
    /// Create from request
    pub fn new(bpan: String, req: BatteryDescriptorRequest) -> Self {
        let battery = BatteryDescriptor {
            id: uuid::Uuid::new_v4(),
            bpan,
            capacity_kwh: req.capacity_kwh,
            nominal_voltage_v: req.nominal_voltage_v,
            nominal_current_a: req.nominal_current_a,
            chemistry_type: req.chemistry_type,
            cell_type: req.cell_type,
            cell_count: req.cell_count,
            cell_voltage_nominal_v: req.cell_voltage_nominal_v,
            manufacturer_id: req.manufacturer_id,
            manufacturing_country: req.manufacturing_country,
            manufacturing_facility: req.manufacturing_facility,
            manufacture_date: req.manufacture_date,
            declared_cycle_life: req.declared_cycle_life,
            warranty_years: req.warranty_years,
            registered_at: Utc::now(),
            battery_hash: String::new(),
        };

        // Compute hash
        let mut computed = battery.clone();
        computed.battery_hash = Self::compute_hash(&battery);
        computed
    }

    /// Compute immutable hash
    pub fn compute_hash(battery: &BatteryDescriptor) -> String {
        let mut hasher = Sha256::new();
        hasher.update(battery.bpan.as_bytes());
        hasher.update(battery.capacity_kwh.to_le_bytes());
        hasher.update(battery.nominal_voltage_v.to_le_bytes());
        hasher.update(battery.chemistry_type.as_bytes());
        hasher.update(battery.cell_type.as_bytes());
        hasher.update(battery.manufacturer_id.as_bytes());
        hasher.update(battery.manufacturing_country.as_bytes());
        hasher.update(battery.manufacture_date.as_bytes());
        hasher.update(battery.registered_at.to_rfc3339().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify hash integrity
    pub fn verify_hash_integrity(&self) -> bool {
        self.battery_hash == Self::compute_hash(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryDescriptorRequest {
    pub capacity_kwh: f32,
    pub nominal_voltage_v: f32,
    pub nominal_current_a: f32,
    pub chemistry_type: String,
    pub cell_type: String,
    pub cell_count: i32,
    pub cell_voltage_nominal_v: f32,
    pub manufacturer_id: uuid::Uuid,
    pub manufacturing_country: String,
    pub manufacturing_facility: String,
    pub manufacture_date: String,
    pub declared_cycle_life: i32,
    pub warranty_years: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_descriptor_hash() {
        let req = BatteryDescriptorRequest {
            capacity_kwh: 30.0,
            nominal_voltage_v: 307.0,
            nominal_current_a: 100.0,
            chemistry_type: "NMC".to_string(),
            cell_type: "21700".to_string(),
            cell_count: 95,
            cell_voltage_nominal_v: 3.7,
            manufacturer_id: uuid::Uuid::new_v4(),
            manufacturing_country: "Korea".to_string(),
            manufacturing_facility: "Factory-8".to_string(),
            manufacture_date: "2025-04-17".to_string(),
            declared_cycle_life: 500000,
            warranty_years: 8,
        };

        let mut battery = BatteryDescriptor::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            req,
        );
        battery.battery_hash = BatteryDescriptor::compute_hash(&battery);

        assert!(battery.verify_hash_integrity());
    }
}
