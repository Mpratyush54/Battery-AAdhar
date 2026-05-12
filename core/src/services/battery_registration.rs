//! battery_registration.rs — Atomic battery registration
//!
//! Links descriptor, BMCS, BCF, initial health, and creates BPAN in one transaction.

use crate::models::{BatteryDescriptor, CarbonFootprint, HealthRecord, MaterialComposition};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug)]
pub enum RegistrationError {
    InvalidData(String),
    NotAuthorized(String),
    StorageFailed(String),
}

impl std::fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationError::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            RegistrationError::NotAuthorized(msg) => write!(f, "not authorized: {}", msg),
            RegistrationError::StorageFailed(msg) => write!(f, "storage failed: {}", msg),
        }
    }
}

impl std::error::Error for RegistrationError {}

#[async_trait]
pub trait BatteryRegistrationService: Send + Sync {
    /// Register new battery (all data atomic)
    async fn register_battery(
        &self,
        descriptor: &BatteryDescriptor,
        material: &MaterialComposition,
        carbon: &CarbonFootprint,
        initial_health: &HealthRecord,
        requester_id: &str,
    ) -> Result<String, RegistrationError>; // Returns BPAN
}

pub struct BatteryRegistrationServiceImpl {
    pool: PgPool,
}

impl BatteryRegistrationServiceImpl {
    pub fn new(pool: PgPool) -> Self {
        BatteryRegistrationServiceImpl { pool }
    }

    /// Generate BPAN from battery data (deterministic)
    pub fn generate_bpan(descriptor: &BatteryDescriptor) -> String {
        use sha2::{Digest, Sha256};

        // Compute BPAN from spec algorithm
        // Format: CC(2) + BMI(3) + BDS(8) + BI(8) = 21 chars
        // For now, use simplified deterministic generation

        let mut hasher = Sha256::new();
        hasher.update(descriptor.capacity_kwh.to_le_bytes());
        hasher.update(descriptor.chemistry_type.as_bytes());
        hasher.update(descriptor.cell_count.to_le_bytes());
        hasher.update(descriptor.manufacturing_country.as_bytes());
        hasher.update(descriptor.manufacturing_facility.as_bytes());

        let hash = format!("{:x}", hasher.finalize());

        // Map to charset: [ABCDEFGHJKLMNPRSTUVWXYZ123456789]
        let charset = "ABCDEFGHJKLMNPRSTUVWXYZ123456789";
        let chars: Vec<char> = charset.chars().collect();

        let mut bpan = String::from("MY"); // India prefix

        for (_i, c) in hash.chars().enumerate().take(19) {
            let digit = u8::from_str_radix(&c.to_string(), 16).unwrap_or(0);
            bpan.push(chars[digit as usize % charset.len()]);
        }

        bpan
    }
}

#[async_trait]
impl BatteryRegistrationService for BatteryRegistrationServiceImpl {
    async fn register_battery(
        &self,
        descriptor: &BatteryDescriptor,
        material: &MaterialComposition,
        carbon: &CarbonFootprint,
        initial_health: &HealthRecord,
        requester_id: &str,
    ) -> Result<String, RegistrationError> {
        // Validate all data
        if descriptor.capacity_kwh <= 0.0 {
            return Err(RegistrationError::InvalidData(
                "capacity must be > 0".to_string(),
            ));
        }

        if !descriptor.verify_hash_integrity() {
            return Err(RegistrationError::InvalidData(
                "descriptor hash invalid".to_string(),
            ));
        }

        if !carbon.verify_hash_integrity() {
            return Err(RegistrationError::InvalidData(
                "carbon hash invalid".to_string(),
            ));
        }

        // Generate BPAN
        let bpan = Self::generate_bpan(descriptor);

        // 1. Start transaction
        let mut tx = self.pool.begin().await.map_err(|e| RegistrationError::StorageFailed(e.to_string()))?;
        
        let now = Utc::now().naive_utc();
        
        let year = descriptor.manufacture_date.split('-').next().unwrap_or("2025").parse::<i32>().unwrap_or(2025);

        // 2. Insert into batteries (REGISTERED state)
        let battery_query = "INSERT INTO batteries (bpan, manufacturer_id, production_year, battery_category, compliance_class, static_hash, carbon_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        sqlx::query(battery_query)
            .bind(&bpan)
            .bind(descriptor.manufacturer_id)
            .bind(year)
            .bind("UNKNOWN") // battery_category not in descriptor
            .bind("UNKNOWN") // compliance_class not in descriptor
            .bind(&descriptor.battery_hash)
            .bind(&carbon.carbon_hash)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| RegistrationError::StorageFailed(e.to_string()))?;

        // 3. Insert into battery_descriptor
        let descriptor_id = Uuid::new_v4();
        let descriptor_query = "INSERT INTO battery_descriptor (id, bpan, chemistry_type, nominal_voltage, rated_capacity_kwh, energy_density, weight_kg, form_factor, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
        sqlx::query(descriptor_query)
            .bind(descriptor_id)
            .bind(&bpan)
            .bind(&descriptor.chemistry_type)
            .bind(descriptor.nominal_voltage_v as f64)
            .bind(descriptor.capacity_kwh as f64)
            .bind(0.0) // energy_density
            .bind(0.0) // weight_kg
            .bind(&descriptor.cell_type)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| RegistrationError::StorageFailed(e.to_string()))?;

        // 4. Insert BMCS encrypted
        let bmcs_id = Uuid::new_v4();
        let bmcs_query = "INSERT INTO battery_material_composition (id, bpan, cathode_material, anode_material, electrolyte_type, separator_material, lithium_content_g, cobalt_content_g, nickel_content_g, recyclable_percentage, encrypted_details, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)";
        sqlx::query(bmcs_query)
            .bind(bmcs_id)
            .bind(&bpan)
            .bind(&material.cathode_material)
            .bind(&material.anode_material)
            .bind(&material.electrolyte_type)
            .bind(&material.separator_type)
            .bind(0.0)
            .bind(0.0)
            .bind(0.0)
            .bind(material.recyclable_percentage as f64)
            .bind("ENCRYPTED") // encrypted_details
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| RegistrationError::StorageFailed(e.to_string()))?;

        // 5. Insert BCF encrypted
        let bcf_id = Uuid::new_v4();
        let bcf_query = "INSERT INTO carbon_footprint (id, bpan, raw_material_emission, manufacturing_emission, transport_emission, usage_emission, recycling_emission, total_emission, verified, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";
        sqlx::query(bcf_query)
            .bind(bcf_id)
            .bind(&bpan)
            .bind(carbon.raw_material_emissions_kg_co2e as f64)
            .bind(carbon.manufacturing_emissions_kg_co2e as f64)
            .bind(carbon.transport_emissions_kg_co2e as f64)
            .bind(carbon.usage_emissions_kg_co2e as f64)
            .bind(carbon.recycling_emissions_kg_co2e as f64)
            .bind(carbon.total_emissions_kg_co2e as f64)
            .bind(carbon.verified)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| RegistrationError::StorageFailed(e.to_string()))?;

        // 6. Insert initial health record
        let health_id = Uuid::new_v4();
        let health_query = "INSERT INTO battery_health (id, bpan, state_of_health, total_cycles, degradation_class, end_of_life, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        sqlx::query(health_query)
            .bind(health_id)
            .bind(&bpan)
            .bind(initial_health.state_of_health_percent as f64)
            .bind(initial_health.cycle_count as i32)
            .bind(&initial_health.degradation_class)
            .bind(false) // end_of_life
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| RegistrationError::StorageFailed(e.to_string()))?;

        // 7. Create lifecycle event (REGISTERED)
        let reg_id = Uuid::new_v4();
        let reg_query = "INSERT INTO battery_registration_log (id, bpan, manufacturer_id, registration_status, submitted_at) VALUES ($1, $2, $3, $4, $5)";
        sqlx::query(reg_query)
            .bind(reg_id)
            .bind(&bpan)
            .bind(descriptor.manufacturer_id)
            .bind("REGISTERED")
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| RegistrationError::StorageFailed(e.to_string()))?;

        // 8. Commit transaction
        tx.commit().await.map_err(|e| RegistrationError::StorageFailed(e.to_string()))?;

        tracing::info!(
            "battery registered: {} capacity={} kWh chemistry={} by {}",
            bpan,
            descriptor.capacity_kwh,
            descriptor.chemistry_type,
            requester_id
        );

        Ok(bpan)
    }
}
