//! carbon.rs — Battery Carbon Footprint (BCF) data models
//!
//! Represents Table 5 from spec Annexure II.
//! 5-stage emissions model with integrity hashing.

use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

/// Raw carbon footprint data (5 stages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonFootprint {
    pub bpan: String,

    // === Stage 1: Raw Material Extraction ===
    pub raw_material_emissions_kg_co2e: f32, // kg CO₂e to extract and process raw materials
    pub raw_material_source_country: String, // Origin of lithium, cobalt, nickel, etc.
    pub mining_method: String,
    // "Hard Rock Mining", "Brine Evaporation", etc.

    // === Stage 2: Manufacturing ===
    pub manufacturing_emissions_kg_co2e: f32,
    // kg CO₂e for cell + pack production
    pub manufacturing_location: String,
    // Factory location (affects grid emissions)
    pub factory_energy_source: String,
    // "Coal", "NG", "Renewable", "Mixed"
    pub cell_production_method: String,
    // "Wet Coating", "Dry Coating", etc.

    // === Stage 3: Transport ===
    pub transport_emissions_kg_co2e: f32,
    // kg CO₂e for logistics to market
    pub transport_distance_km: f32,
    // Distance traveled (port → customer)
    pub transport_mode: String,
    // "Sea", "Air", "Rail", "Truck"
    pub transport_packaging: String,
    // Packaging type (affects return trip emissions)

    // === Stage 4: Usage Phase ===
    pub usage_emissions_kg_co2e: f32,
    // kg CO₂e per kWh over battery lifetime
    pub usage_years: i32,
    // Assumed useful life (typically 8–10 years)
    pub usage_grid_emissions_factor: f32, // g CO₂e per kWh (grid mix)
    pub usage_annual_km: i32,
    // Assumed annual vehicle km (EV context)

    // === Stage 5: Recycling/EOL ===
    pub recycling_emissions_kg_co2e: f32,
    // kg CO₂e for recycling + material recovery
    pub recycling_recovery_rate: f32,
    // % of materials recovered (0–100)
    pub recycling_avoided_mining: f32,
    // Negative emissions (avoided virgin mining)
    pub recycling_method: String,
    // "Mechanical", "Hydrometallurgical", "Pyrometallurgical"

    // === Totals ===
    pub total_emissions_kg_co2e: f32, // Sum of all 5 stages (computed)
    pub emissions_per_kwh: f32,       // Normalized to capacity
    pub carbon_hash: String,          // SHA256(stage1||stage2||stage3||stage4||stage5||timestamp)

    // === Metadata ===
    pub submitted_by: String, // Manufacturer ID
    pub submitted_at: DateTime<Utc>,
    pub submitted_version: i32,
    pub verified: bool,
    pub verified_by: Option<String>, // Verifier ID
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_standard: Option<String>, // "ISO 14040", "PEF", "EU ETS"
}

impl CarbonFootprint {
    /// Create from request
    pub fn from_request(bpan: String, data: CarbonFootprintRequest, submitted_by: String) -> Self {
        // Compute total emissions (simple sum, in production may use weighted model)
        let total_emissions_kg_co2e = data.raw_material_emissions_kg_co2e
            + data.manufacturing_emissions_kg_co2e
            + data.transport_emissions_kg_co2e
            + data.usage_emissions_kg_co2e
            + data.recycling_emissions_kg_co2e;

        // Compute emissions per kWh (requires battery capacity — would fetch from DB)
        // For now, assume 30 kWh capacity (from pilot example)
        let emissions_per_kwh = total_emissions_kg_co2e / 30.0;

        // Compute carbon hash
        let now = Utc::now();
        let carbon_hash = Self::compute_hash(
            data.raw_material_emissions_kg_co2e,
            data.manufacturing_emissions_kg_co2e,
            data.transport_emissions_kg_co2e,
            data.usage_emissions_kg_co2e,
            data.recycling_emissions_kg_co2e,
            &now,
        );

        CarbonFootprint {
            bpan,
            raw_material_emissions_kg_co2e: data.raw_material_emissions_kg_co2e,
            raw_material_source_country: data.raw_material_source_country,
            mining_method: data.mining_method,
            manufacturing_emissions_kg_co2e: data.manufacturing_emissions_kg_co2e,
            manufacturing_location: data.manufacturing_location,
            factory_energy_source: data.factory_energy_source,
            cell_production_method: data.cell_production_method,
            transport_emissions_kg_co2e: data.transport_emissions_kg_co2e,
            transport_distance_km: data.transport_distance_km,
            transport_mode: data.transport_mode,
            transport_packaging: data.transport_packaging,
            usage_emissions_kg_co2e: data.usage_emissions_kg_co2e,
            usage_years: data.usage_years,
            usage_grid_emissions_factor: data.usage_grid_emissions_factor,
            usage_annual_km: data.usage_annual_km,
            recycling_emissions_kg_co2e: data.recycling_emissions_kg_co2e,
            recycling_recovery_rate: data.recycling_recovery_rate,
            recycling_avoided_mining: data.recycling_avoided_mining,
            recycling_method: data.recycling_method,
            total_emissions_kg_co2e,
            emissions_per_kwh,
            carbon_hash,
            submitted_by,
            submitted_at: now,
            submitted_version: 1,
            verified: false,
            verified_by: None,
            verified_at: None,
            verification_standard: None,
        }
    }

    /// Compute SHA256 hash of all emissions + timestamp
    pub fn compute_hash(
        stage1: f32,
        stage2: f32,
        stage3: f32,
        stage4: f32,
        stage5: f32,
        timestamp: &DateTime<Utc>,
    ) -> String {
        let mut hasher = Sha256::new();

        // Hash all stages as bytes + timestamp
        hasher.update(stage1.to_le_bytes());
        hasher.update(stage2.to_le_bytes());
        hasher.update(stage3.to_le_bytes());
        hasher.update(stage4.to_le_bytes());
        hasher.update(stage5.to_le_bytes());
        hasher.update(timestamp.to_rfc3339().as_bytes());

        format!("{:x}", hasher.finalize())
    }

    /// Recompute hash (for verification/tamper detection)
    pub fn recompute_hash(&self) -> String {
        Self::compute_hash(
            self.raw_material_emissions_kg_co2e,
            self.manufacturing_emissions_kg_co2e,
            self.transport_emissions_kg_co2e,
            self.usage_emissions_kg_co2e,
            self.recycling_emissions_kg_co2e,
            &self.submitted_at,
        )
    }

    /// Verify hash integrity
    pub fn verify_hash_integrity(&self) -> bool {
        self.carbon_hash == self.recompute_hash()
    }

    /// Compute total (sum of 5 stages)
    pub fn compute_total(&mut self) {
        self.total_emissions_kg_co2e = self.raw_material_emissions_kg_co2e
            + self.manufacturing_emissions_kg_co2e
            + self.transport_emissions_kg_co2e
            + self.usage_emissions_kg_co2e
            + self.recycling_emissions_kg_co2e;
    }

    /// Convert to bytes for encryption
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Request payload for submitting carbon footprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonFootprintRequest {
    pub raw_material_emissions_kg_co2e: f32,
    pub raw_material_source_country: String,
    pub mining_method: String,
    pub manufacturing_emissions_kg_co2e: f32,
    pub manufacturing_location: String,
    pub factory_energy_source: String,
    pub cell_production_method: String,
    pub transport_emissions_kg_co2e: f32,
    pub transport_distance_km: f32,
    pub transport_mode: String,
    pub transport_packaging: String,
    pub usage_emissions_kg_co2e: f32,
    pub usage_years: i32,
    pub usage_grid_emissions_factor: f32,
    pub usage_annual_km: i32,
    pub recycling_emissions_kg_co2e: f32,
    pub recycling_recovery_rate: f32,
    pub recycling_avoided_mining: f32,
    pub recycling_method: String,
}

/// Public-only view (for consumers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonFootprintPublic {
    pub bpan: String,
    pub total_emissions_kg_co2e: f32,
    pub emissions_per_kwh: f32,
    pub verified: bool,
    pub verified_by: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_standard: Option<String>,
}

impl From<&CarbonFootprint> for CarbonFootprintPublic {
    fn from(cf: &CarbonFootprint) -> Self {
        CarbonFootprintPublic {
            bpan: cf.bpan.clone(),
            total_emissions_kg_co2e: cf.total_emissions_kg_co2e,
            emissions_per_kwh: cf.emissions_per_kwh,
            verified: cf.verified,
            verified_by: cf.verified_by.clone(),
            verified_at: cf.verified_at,
            verification_standard: cf.verification_standard.clone(),
        }
    }
}

/// Carbon comparison result (battery A vs battery B)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonComparison {
    pub bpan_a: String,
    pub bpan_b: String,
    pub stage1_delta: f32,
    pub stage2_delta: f32,
    pub stage3_delta: f32,
    pub stage4_delta: f32,
    pub stage5_delta: f32,
    pub total_delta: f32,
    pub emissions_per_kwh_delta: f32,
    pub bpan_a_lower: bool, // true if A has lower total emissions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn realistic_carbon_data() -> CarbonFootprintRequest {
        CarbonFootprintRequest {
            raw_material_emissions_kg_co2e: 45.0, // kg CO₂e
            raw_material_source_country: "Indonesia".to_string(),
            mining_method: "Brine Evaporation".to_string(),
            manufacturing_emissions_kg_co2e: 35.0,
            manufacturing_location: "China".to_string(),
            factory_energy_source: "Renewable".to_string(),
            cell_production_method: "Wet Coating".to_string(),
            transport_emissions_kg_co2e: 12.0,
            transport_distance_km: 15000.0,
            transport_mode: "Sea".to_string(),
            transport_packaging: "Recyclable carton".to_string(),
            usage_emissions_kg_co2e: 80.0,
            usage_years: 8,
            usage_grid_emissions_factor: 500.0,
            usage_annual_km: 15000,
            recycling_emissions_kg_co2e: -15.0, // Negative (avoided mining)
            recycling_recovery_rate: 85.0,
            recycling_avoided_mining: 30.0,
            recycling_method: "Hydrometallurgical".to_string(),
        }
    }

    #[test]
    fn test_carbon_footprint_total() {
        let req = realistic_carbon_data();
        let mut cf = CarbonFootprint::from_request(
            "MY008A6FKKKLC1DH80001".to_string(),
            req,
            "mfr-001".to_string(),
        );

        // Total should be sum of 5 stages
        let expected_total = 45.0 + 35.0 + 12.0 + 80.0 + (-15.0);
        assert_eq!(cf.total_emissions_kg_co2e, expected_total);

        // Recompute should match
        cf.compute_total();
        assert_eq!(cf.total_emissions_kg_co2e, expected_total);
    }

    #[test]
    fn test_carbon_hash_integrity() {
        let req = realistic_carbon_data();
        let cf = CarbonFootprint::from_request(
            "MY008A6FKKKLC1DH80001".to_string(),
            req,
            "mfr-001".to_string(),
        );

        // Hash should verify
        assert!(cf.verify_hash_integrity());

        // Tamper detection: change one field
        let mut cf_tampered = cf.clone();
        cf_tampered.raw_material_emissions_kg_co2e = 50.0; // Changed!

        // Hash should NOT verify
        assert!(!cf_tampered.verify_hash_integrity());
    }

    #[test]
    fn test_carbon_footprint_roundtrip() {
        let req = realistic_carbon_data();
        let cf = CarbonFootprint::from_request(
            "MY008A6FKKKLC1DH80001".to_string(),
            req,
            "mfr-001".to_string(),
        );

        let bytes = cf.to_bytes().expect("serialize failed");
        let recovered = CarbonFootprint::from_bytes(&bytes).expect("deserialize failed");

        assert_eq!(cf.bpan, recovered.bpan);
        assert_eq!(
            cf.total_emissions_kg_co2e,
            recovered.total_emissions_kg_co2e
        );
        assert_eq!(cf.carbon_hash, recovered.carbon_hash);
    }

    #[test]
    fn test_carbon_comparison() {
        let req_a = realistic_carbon_data();
        let cf_a = CarbonFootprint::from_request(
            "MY008A6FKKKLC1DH80001".to_string(),
            req_a,
            "mfr-001".to_string(),
        );

        let mut req_b = realistic_carbon_data();
        req_b.manufacturing_emissions_kg_co2e = 40.0; // Slightly higher
        let cf_b = CarbonFootprint::from_request(
            "MY008A6FKKKLC1DH80002".to_string(),
            req_b,
            "mfr-001".to_string(),
        );

        let comparison = CarbonComparison {
            bpan_a: cf_a.bpan.clone(),
            bpan_b: cf_b.bpan.clone(),
            stage1_delta: cf_a.raw_material_emissions_kg_co2e - cf_b.raw_material_emissions_kg_co2e,
            stage2_delta: cf_a.manufacturing_emissions_kg_co2e
                - cf_b.manufacturing_emissions_kg_co2e,
            stage3_delta: cf_a.transport_emissions_kg_co2e - cf_b.transport_emissions_kg_co2e,
            stage4_delta: cf_a.usage_emissions_kg_co2e - cf_b.usage_emissions_kg_co2e,
            stage5_delta: cf_a.recycling_emissions_kg_co2e - cf_b.recycling_emissions_kg_co2e,
            total_delta: cf_a.total_emissions_kg_co2e - cf_b.total_emissions_kg_co2e,
            emissions_per_kwh_delta: cf_a.emissions_per_kwh - cf_b.emissions_per_kwh,
            bpan_a_lower: cf_a.total_emissions_kg_co2e < cf_b.total_emissions_kg_co2e,
        };

        // B has higher manufacturing, so A should be lower
        assert!(comparison.bpan_a_lower);
        assert!(comparison.stage2_delta < 0.0); // A manufactured with less emissions
    }
}
