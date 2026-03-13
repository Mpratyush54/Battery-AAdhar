use chrono::Utc;
use sqlx::{Pool, Postgres};
use tracing::{info, error, instrument};
use uuid::Uuid;

use crate::errors::{BpaError, BpaResult};
use crate::services::bpan_generator::BpanGenerator;
use crate::services::encryption::EncryptionService;
use crate::services::hash_chain::HashChainService;
use crate::services::validation::ValidationService;

/// Orchestrates the full battery registration workflow per Phase 1 of the BPA guideline.
///
/// Registration flow:
/// 1. Validate all input fields
/// 2. Generate BPAN (21-character alphanumeric)
/// 3. Encrypt sensitive identifiers (serial, batch, factory code)
/// 4. Compute static data hash
/// 5. Insert battery record
/// 6. Insert battery identifiers (encrypted)
/// 7. Insert battery descriptor
/// 8. Create registration log entry
/// 9. Audit log everything in a single transaction
#[derive(Clone)]
pub struct RegistrationService {
    pool: Pool<Postgres>,
    encryption: EncryptionService,
}

impl RegistrationService {
    pub fn new(pool: Pool<Postgres>, encryption: EncryptionService) -> Self {
        Self { pool, encryption }
    }

    /// Register a new battery in the system.
    /// This is the primary entry point for manufacturers/importers.
    #[instrument(name = "register_battery", skip(self, request))]
    pub async fn register_battery(
        &self,
        request: BatteryRegistrationRequest,
        actor_id: Uuid,
    ) -> BpaResult<BatteryRegistrationResponse> {
        // Step 1: Validate all inputs
        ValidationService::validate_non_empty("manufacturer_code", &request.manufacturer_code)?;
        ValidationService::validate_chemistry_type(&request.chemistry_type)?;
        ValidationService::validate_battery_category(&request.battery_category)?;
        ValidationService::validate_compliance_class(&request.compliance_class)?;
        ValidationService::validate_capacity(request.rated_capacity_kwh)?;
        ValidationService::validate_voltage(request.nominal_voltage)?;
        ValidationService::validate_energy_density(request.energy_density)?;
        ValidationService::validate_weight(request.weight_kg)?;
        ValidationService::validate_form_factor(&request.form_factor)?;
        ValidationService::validate_non_empty("serial_number", &request.serial_number)?;

        // Step 2: Map chemistry type to code
        let chemistry_code = Self::map_chemistry_to_code(&request.chemistry_type)?;
        let category_code = Self::map_category_to_code(&request.battery_category)?;

        // Step 3: Generate BPAN
        let bpan = BpanGenerator::generate(
            &request.manufacturer_code,
            &chemistry_code,
            &category_code,
            request.rated_capacity_kwh,
            &request.serial_number[..8.min(request.serial_number.len())],
            request.production_year,
            &request.sequence_number,
        )?;

        // Step 4: Encrypt sensitive identifiers
        let encrypted_serial = self.encryption.encrypt(&request.serial_number)?;
        let encrypted_batch = self.encryption.encrypt(&request.batch_number)?;
        let encrypted_factory = self.encryption.encrypt(&request.factory_code)?;

        // Step 5: Compute hashes
        let static_hash = HashChainService::compute_static_hash(
            &bpan,
            &request.chemistry_type,
            request.nominal_voltage,
            request.rated_capacity_kwh,
            &request.form_factor,
        );
        let carbon_hash = "PENDING".to_string(); // Carbon footprint comes in Phase 3

        let now = Utc::now().naive_utc();

        // Step 6: Execute everything in a single transaction
        let mut tx = self.pool.begin().await?;

        // 6a: Insert battery record
        let battery_query = "INSERT INTO batteries (bpan, manufacturer_id, production_year, battery_category, compliance_class, static_hash, carbon_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        sqlx::query(battery_query)
            .bind(&bpan)
            .bind(&request.manufacturer_id)
            .bind(request.production_year as i32)
            .bind(&request.battery_category)
            .bind(&request.compliance_class)
            .bind(&static_hash)
            .bind(&carbon_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert battery: {:?}", e);
                BpaError::Database(e)
            })?;

        // 6b: Insert battery identifiers
        let id_uuid = Uuid::new_v4();
        let identifiers_query = "INSERT INTO battery_identifiers (id, bpan, cipher_algorithm, cipher_version, encrypted_serial_number, encrypted_batch_number, encrypted_factory_code, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        let _query = sqlx::query(identifiers_query)
            .bind(&id_uuid)
            .bind(&bpan)
            .bind("AES-256-GCM")
            .bind(1i32)
            .bind(&encrypted_serial)
            .bind(&encrypted_batch)
            .bind(&encrypted_factory)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert battery_identifiers: {:?}", e);
                BpaError::Database(e)
            })?;

        // 6c: Insert battery descriptor
        let desc_uuid = Uuid::new_v4();
        let descriptor_query = "INSERT INTO battery_descriptor (id, bpan, chemistry_type, nominal_voltage, rated_capacity_kwh, energy_density, weight_kg, form_factor, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
        sqlx::query(descriptor_query)
            .bind(&desc_uuid)
            .bind(&bpan)
            .bind(&request.chemistry_type)
            .bind(request.nominal_voltage)
            .bind(request.rated_capacity_kwh)
            .bind(request.energy_density)
            .bind(request.weight_kg)
            .bind(&request.form_factor)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert battery_descriptor: {:?}", e);
                BpaError::Database(e)
            })?;

        // 6d: Insert initial battery_health record
        let health_uuid = Uuid::new_v4();
        let health_query = "INSERT INTO battery_health (id, bpan, state_of_health, total_cycles, degradation_class, end_of_life, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        sqlx::query(health_query)
            .bind(&health_uuid)
            .bind(&bpan)
            .bind(100.0_f64)  // New battery starts at 100% SoH
            .bind(0i32)       // Zero cycles
            .bind("A")        // Best degradation class
            .bind(false)      // Not end of life
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert battery_health: {:?}", e);
                BpaError::Database(e)
            })?;

        // 6e: Insert registration log
        let reg_uuid = Uuid::new_v4();
        let reg_query = "INSERT INTO battery_registration_log (id, bpan, manufacturer_id, registration_status, submitted_at, approved_at, approved_by) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        sqlx::query(reg_query)
            .bind(&reg_uuid)
            .bind(&bpan)
            .bind(&request.manufacturer_id)
            .bind("PENDING")
            .bind(&now)
            .bind(&now)  // Will be updated on approval
            .bind(&actor_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert battery_registration_log: {:?}", e);
                BpaError::Database(e)
            })?;

        // 6f: Audit log
        let audit_uuid = Uuid::new_v4();
        let ts_str = now.to_string();
        let entry_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "REGISTER_BATTERY",
            &bpan,
            &actor_id.to_string(),
            &ts_str,
        );
        let audit_query = "INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        sqlx::query(audit_query)
            .bind(&audit_uuid)
            .bind(&actor_id)
            .bind("REGISTER_BATTERY")
            .bind(&bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&entry_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert audit_logs: {:?}", e);
                BpaError::Database(e)
            })?;

        // Commit everything
        tx.commit().await?;

        info!("Battery registered successfully with BPAN: {}", bpan);

        Ok(BatteryRegistrationResponse {
            bpan,
            static_hash,
            registration_id: reg_uuid,
            status: "PENDING".to_string(),
        })
    }

    /// Approve a pending battery registration.
    #[instrument(name = "approve_registration", skip(self))]
    pub async fn approve_registration(
        &self,
        registration_id: Uuid,
        approver_id: Uuid,
    ) -> BpaResult<()> {
        let now = Utc::now().naive_utc();

        let mut tx = self.pool.begin().await?;

        let update_query = "UPDATE battery_registration_log SET registration_status = $1, approved_at = $2, approved_by = $3 WHERE id = $4 AND registration_status = 'PENDING'";
        let result = sqlx::query(update_query)
            .bind("APPROVED")
            .bind(&now)
            .bind(&approver_id)
            .bind(&registration_id)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(BpaError::NotFound(format!(
                "No pending registration found with id {}",
                registration_id
            )));
        }

        // Audit
        let audit_uuid = Uuid::new_v4();
        let ts_str = now.to_string();
        let entry_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "APPROVE_REGISTRATION",
            &registration_id.to_string(),
            &approver_id.to_string(),
            &ts_str,
        );
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_uuid)
            .bind(&approver_id)
            .bind("APPROVE_REGISTRATION")
            .bind(&registration_id.to_string())
            .bind(&HashChainService::genesis_hash())
            .bind(&entry_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Registration {} approved by {}", registration_id, approver_id);
        Ok(())
    }

    /// Reject a pending battery registration.
    #[instrument(name = "reject_registration", skip(self))]
    pub async fn reject_registration(
        &self,
        registration_id: Uuid,
        rejector_id: Uuid,
    ) -> BpaResult<()> {
        let now = Utc::now().naive_utc();

        let update_query = "UPDATE battery_registration_log SET registration_status = $1, approved_at = $2, approved_by = $3 WHERE id = $4 AND registration_status = 'PENDING'";
        let result = sqlx::query(update_query)
            .bind("REJECTED")
            .bind(&now)
            .bind(&rejector_id)
            .bind(&registration_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(BpaError::NotFound(format!(
                "No pending registration found with id {}",
                registration_id
            )));
        }

        info!("Registration {} rejected by {}", registration_id, rejector_id);
        Ok(())
    }

    // --- Private helpers ---

    fn map_chemistry_to_code(chemistry: &str) -> BpaResult<String> {
        match chemistry.to_uppercase().as_str() {
            "LFP" => Ok("LF".into()),
            "NMC" => Ok("NM".into()),
            "NCA" => Ok("NC".into()),
            "LTO" => Ok("LT".into()),
            "SOLID-STATE" => Ok("SS".into()),
            "NAION" => Ok("NA".into()),
            "OTHER" => Ok("OT".into()),
            _ => Err(BpaError::BpanFormat(format!(
                "Cannot map chemistry '{}' to BPAN code",
                chemistry
            ))),
        }
    }

    fn map_category_to_code(category: &str) -> BpaResult<String> {
        match category.to_uppercase().as_str() {
            "EV-L" => Ok("EL".into()),
            "EV-M" => Ok("EM".into()),
            "EV-N" => Ok("EN".into()),
            "INDUSTRIAL" => Ok("IN".into()),
            "ESS" => Ok("ES".into()),
            _ => Err(BpaError::BpanFormat(format!(
                "Cannot map category '{}' to BPAN code",
                category
            ))),
        }
    }
}

/// Request payload for battery registration.
#[derive(Debug, Clone)]
pub struct BatteryRegistrationRequest {
    pub manufacturer_id: Uuid,
    pub manufacturer_code: String,   // 3-char regulator-assigned code
    pub chemistry_type: String,      // LFP, NMC, NCA, etc.
    pub battery_category: String,    // EV-L, EV-M, EV-N, Industrial, ESS
    pub compliance_class: String,    // AIS-156, etc.
    pub nominal_voltage: f64,
    pub rated_capacity_kwh: f64,
    pub energy_density: f64,
    pub weight_kg: f64,
    pub form_factor: String,
    pub serial_number: String,       // 8+ char alphanumeric
    pub batch_number: String,
    pub factory_code: String,
    pub production_year: u16,
    pub sequence_number: String,     // 2-char sequence
}

/// Response payload after successful registration.
#[derive(Debug, Clone)]
pub struct BatteryRegistrationResponse {
    pub bpan: String,
    pub static_hash: String,
    pub registration_id: Uuid,
    pub status: String,
}
