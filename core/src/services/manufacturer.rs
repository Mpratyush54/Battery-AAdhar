use chrono::Utc;
use sqlx::{Pool, Postgres, Row};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::errors::{BpaError, BpaResult};
use crate::models::Manufacturers;
use crate::services::encryption::EncryptionService;
use crate::services::hash_chain::HashChainService;
use crate::services::key_manager::KeyManagerImpl;

/// Manages manufacturer profiles: registration, code assignment, DEK-based encryption.
///
/// Each manufacturer gets a 3-char uppercase alpha code assigned by the regulator.
/// Profile data is encrypted with a manufacturer-specific DEK derived from the key manager.
#[derive(Clone)]
pub struct ManufacturerService {
    pool: Pool<Postgres>,
    encryption: EncryptionService,
    key_manager: std::sync::Arc<KeyManagerImpl>,
}

/// Payload for registering a new manufacturer.
#[derive(Debug, Clone)]
pub struct RegisterManufacturerRequest {
    pub name: String,
    pub country_code: String,
    pub profile_data: String,
}

/// Response after successful manufacturer registration.
#[derive(Debug, Clone)]
pub struct RegisterManufacturerResponse {
    pub id: Uuid,
    pub manufacturer_code: String,
    pub name: String,
}

/// Manufacturer profile details (after decryption).
#[derive(Debug, Clone)]
pub struct ManufacturerProfile {
    pub id: Uuid,
    pub manufacturer_code: String,
    pub name: String,
    pub country_code: String,
    pub profile_data: String,
    pub created_at: chrono::NaiveDateTime,
}

/// Batch battery registration input row.
#[derive(Debug, Clone)]
pub struct BatteryCsvRow {
    pub chemistry_type: String,
    pub battery_category: String,
    pub compliance_class: String,
    pub nominal_voltage: f64,
    pub rated_capacity_kwh: f64,
    pub energy_density: f64,
    pub weight_kg: f64,
    pub form_factor: String,
    pub serial_number: String,
    pub batch_number: String,
    pub factory_code: String,
    pub production_year: u16,
    pub sequence_number: String,
}

/// Result for a single battery in a batch registration.
#[derive(Debug, Clone)]
pub struct BatteryBatchResult {
    pub bpan: String,
    pub static_hash: String,
    pub status: String,
}

/// Response for batch battery registration.
#[derive(Debug, Clone)]
pub struct BatchRegistrationResponse {
    pub manufacturer_id: Uuid,
    pub total: usize,
    pub batteries: Vec<BatteryBatchResult>,
    pub audit_id: Uuid,
}

impl ManufacturerService {
    pub fn new(
        pool: Pool<Postgres>,
        encryption: EncryptionService,
        key_manager: std::sync::Arc<KeyManagerImpl>,
    ) -> Self {
        Self {
            pool,
            encryption,
            key_manager,
        }
    }

    /// Register a new manufacturer with a regulator-assigned 3-char code.
    /// Profile data is encrypted using a manufacturer-specific DEK.
    #[instrument(name = "register_manufacturer", skip(self, request))]
    pub async fn register_manufacturer(
        &self,
        request: RegisterManufacturerRequest,
        regulator_id: Uuid,
    ) -> BpaResult<RegisterManufacturerResponse> {
        if request.name.trim().is_empty() {
            return Err(BpaError::Validation("manufacturer name is required".into()));
        }
        if request.country_code.len() != 2 {
            return Err(BpaError::Validation("country code must be 2 letters".into()));
        }

        // Check for duplicate name
        let existing = sqlx::query("SELECT id FROM manufacturers WHERE name = $1")
            .bind(&request.name)
            .fetch_optional(&self.pool)
            .await?;
        if existing.is_some() {
            return Err(BpaError::Conflict(format!(
                "manufacturer '{}' already exists",
                request.name
            )));
        }

        let manufacturer_id = Uuid::new_v4();

        // Assign a unique 3-char manufacturer code
        let manufacturer_code = self.assign_manufacturer_code().await?;

        // Derive a manufacturer-specific DEK using key manager
        // Use the manufacturer_code as the "bpan" equivalent in the DEK derivation
        let dek = self
            .key_manager
            .derive_dek_from_code(&manufacturer_code)
            .map_err(|e| BpaError::Encryption(format!("DEK derivation failed: {}", e)))?;

        // Create encryption service with the manufacturer-specific DEK
        let dek_bytes: [u8; 32] = *dek.as_bytes();
        let dek_hex = hex::encode(dek_bytes);
        let mfr_encryption = EncryptionService::new(&dek_hex)?;

        // Encrypt the profile data
        let encrypted_profile = mfr_encryption.encrypt(&request.profile_data)?;

        let now = Utc::now().naive_utc();

        let mut tx = self.pool.begin().await?;

        let insert_query = "INSERT INTO manufacturers (id, manufacturer_code, name, country_code, encrypted_profile, created_at) VALUES ($1, $2, $3, $4, $5, $6)";
        sqlx::query(insert_query)
            .bind(manufacturer_id)
            .bind(&manufacturer_code)
            .bind(&request.name)
            .bind(&request.country_code)
            .bind(&encrypted_profile)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert manufacturer: {:?}", e);
                BpaError::Database(e)
            })?;

        // Audit log
        let audit_uuid = Uuid::new_v4();
        let ts_str = now.to_string();
        let entry_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "REGISTER_MANUFACTURER",
            &manufacturer_code,
            &regulator_id.to_string(),
            &ts_str,
        );
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(audit_uuid)
            .bind(regulator_id)
            .bind("REGISTER_MANUFACTURER")
            .bind(&manufacturer_code)
            .bind(HashChainService::genesis_hash())
            .bind(&entry_hash)
            .bind(now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!(
            "Manufacturer registered: {} (code: {})",
            request.name, manufacturer_code
        );

        Ok(RegisterManufacturerResponse {
            id: manufacturer_id,
            manufacturer_code,
            name: request.name,
        })
    }

    /// Get a manufacturer by ID, decrypting the profile.
    #[instrument(name = "get_manufacturer", skip(self))]
    pub async fn get_manufacturer(&self, id: Uuid) -> BpaResult<ManufacturerProfile> {
        let row = sqlx::query(
            "SELECT id, manufacturer_code, name, country_code, encrypted_profile, created_at FROM manufacturers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| BpaError::NotFound(format!("manufacturer {} not found", id)))?;

        let code: String = row.get("manufacturer_code");
        let encrypted_profile: String = row.get("encrypted_profile");

        // Decrypt profile using manufacturer-specific DEK
        let dek = self
            .key_manager
            .derive_dek_from_code(&code)
            .map_err(|e| BpaError::Encryption(format!("DEK derivation failed: {}", e)))?;
        let dek_bytes: [u8; 32] = *dek.as_bytes();
        let dek_hex = hex::encode(dek_bytes);
        let mfr_encryption = EncryptionService::new(&dek_hex)?;
        let profile_data = mfr_encryption.decrypt(&encrypted_profile)?;

        Ok(ManufacturerProfile {
            id: row.get("id"),
            manufacturer_code: code,
            name: row.get("name"),
            country_code: row.get("country_code"),
            profile_data,
            created_at: row.get("created_at"),
        })
    }

    /// Get a manufacturer by code.
    #[instrument(name = "get_manufacturer_by_code", skip(self))]
    pub async fn get_manufacturer_by_code(&self, code: &str) -> BpaResult<ManufacturerProfile> {
        let row = sqlx::query(
            "SELECT id, manufacturer_code, name, country_code, encrypted_profile, created_at FROM manufacturers WHERE manufacturer_code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| BpaError::NotFound(format!("manufacturer code '{}' not found", code)))?;

        let encrypted_profile: String = row.get("encrypted_profile");

        let dek = self
            .key_manager
            .derive_dek_from_code(code)
            .map_err(|e| BpaError::Encryption(format!("DEK derivation failed: {}", e)))?;
        let dek_bytes: [u8; 32] = *dek.as_bytes();
        let dek_hex = hex::encode(dek_bytes);
        let mfr_encryption = EncryptionService::new(&dek_hex)?;
        let profile_data = mfr_encryption.decrypt(&encrypted_profile)?;

        Ok(ManufacturerProfile {
            id: row.get("id"),
            manufacturer_code: code.to_string(),
            name: row.get("name"),
            country_code: row.get("country_code"),
            profile_data,
            created_at: row.get("created_at"),
        })
    }

    /// List all manufacturers.
    #[instrument(name = "list_manufacturers", skip(self))]
    pub async fn list_manufacturers(&self) -> BpaResult<Vec<Manufacturers>> {
        let rows = sqlx::query_as::<_, Manufacturers>(
            "SELECT id, manufacturer_code, name, country_code, encrypted_profile, created_at FROM manufacturers ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Batch register batteries from CSV data.
    /// All batteries are registered in a single transaction with one audit log entry.
    #[instrument(name = "batch_register_batteries", skip(self, rows))]
    pub async fn batch_register_batteries(
        &self,
        manufacturer_id: Uuid,
        manufacturer_code: &str,
        rows: Vec<BatteryCsvRow>,
        actor_id: Uuid,
    ) -> BpaResult<BatchRegistrationResponse> {
        if rows.is_empty() {
            return Err(BpaError::Validation("batch must contain at least 1 battery".into()));
        }

        use crate::services::bpan_generator::BpanGenerator;
        use crate::services::validation::ValidationService;

        let now = Utc::now().naive_utc();
        let mut tx = self.pool.begin().await?;

        let mut results: Vec<BatteryBatchResult> = Vec::with_capacity(rows.len());

        for row in &rows {
            // Validate inputs
            ValidationService::validate_chemistry_type(&row.chemistry_type)?;
            ValidationService::validate_battery_category(&row.battery_category)?;
            ValidationService::validate_compliance_class(&row.compliance_class)?;
            ValidationService::validate_capacity(row.rated_capacity_kwh)?;
            ValidationService::validate_voltage(row.nominal_voltage)?;
            ValidationService::validate_energy_density(row.energy_density)?;
            ValidationService::validate_weight(row.weight_kg)?;
            ValidationService::validate_form_factor(&row.form_factor)?;
            ValidationService::validate_non_empty("serial_number", &row.serial_number)?;

            let chemistry_code = Self::map_chemistry_to_code_inline(&row.chemistry_type)?;
            let category_code = Self::map_category_to_code_inline(&row.battery_category)?;

            let bpan = BpanGenerator::generate(
                manufacturer_code,
                &chemistry_code,
                &category_code,
                row.rated_capacity_kwh,
                &row.serial_number[..8.min(row.serial_number.len())],
                row.production_year,
                &row.sequence_number,
            )?;

            let encrypted_serial = self.encryption.encrypt(&row.serial_number)?;
            let encrypted_batch = self.encryption.encrypt(&row.batch_number)?;
            let encrypted_factory = self.encryption.encrypt(&row.factory_code)?;

            let static_hash = HashChainService::compute_static_hash(
                &bpan,
                &row.chemistry_type,
                row.nominal_voltage,
                row.rated_capacity_kwh,
                &row.form_factor,
            );

            // Insert battery
            sqlx::query("INSERT INTO batteries (bpan, manufacturer_id, production_year, battery_category, compliance_class, static_hash, carbon_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
                .bind(&bpan)
                .bind(manufacturer_id)
                .bind(row.production_year as i32)
                .bind(&row.battery_category)
                .bind(&row.compliance_class)
                .bind(&static_hash)
                .bind("PENDING")
                .bind(now)
                .execute(&mut *tx)
                .await?;

            // Insert identifiers
            sqlx::query("INSERT INTO battery_identifiers (id, bpan, cipher_algorithm, cipher_version, encrypted_serial_number, encrypted_batch_number, encrypted_factory_code, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
                .bind(Uuid::new_v4())
                .bind(&bpan)
                .bind("AES-256-GCM")
                .bind(1i32)
                .bind(&encrypted_serial)
                .bind(&encrypted_batch)
                .bind(&encrypted_factory)
                .bind(now)
                .execute(&mut *tx)
                .await?;

            // Insert descriptor
            sqlx::query("INSERT INTO battery_descriptor (id, bpan, chemistry_type, nominal_voltage, rated_capacity_kwh, energy_density, weight_kg, form_factor, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
                .bind(Uuid::new_v4())
                .bind(&bpan)
                .bind(&row.chemistry_type)
                .bind(row.nominal_voltage)
                .bind(row.rated_capacity_kwh)
                .bind(row.energy_density)
                .bind(row.weight_kg)
                .bind(&row.form_factor)
                .bind(now)
                .execute(&mut *tx)
                .await?;

            // Insert initial health
            sqlx::query("INSERT INTO battery_health (id, bpan, state_of_health, total_cycles, degradation_class, end_of_life, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                .bind(Uuid::new_v4())
                .bind(&bpan)
                .bind(100.0_f64)
                .bind(0i32)
                .bind("A")
                .bind(false)
                .bind(now)
                .execute(&mut *tx)
                .await?;

            // Insert registration log
            sqlx::query("INSERT INTO battery_registration_log (id, bpan, manufacturer_id, registration_status, submitted_at, approved_at, approved_by) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                .bind(Uuid::new_v4())
                .bind(&bpan)
                .bind(manufacturer_id)
                .bind("PENDING")
                .bind(now)
                .bind(now)
                .bind(actor_id)
                .execute(&mut *tx)
                .await?;

            results.push(BatteryBatchResult {
                bpan: bpan.clone(),
                static_hash: static_hash.clone(),
                status: "PENDING".to_string(),
            });

            info!("Battery registered in batch: {}", bpan);
        }

        // Single audit log entry for entire batch
        let audit_uuid = Uuid::new_v4();
        let bpan_list: Vec<String> = results.iter().map(|r| r.bpan.clone()).collect();
        let bpan_csv = bpan_list.join(",");
        let ts_str = now.to_string();
        let entry_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "BATCH_REGISTER_BATTERIES",
            &format!("batch_{}_count_{}", manufacturer_code, rows.len()),
            &actor_id.to_string(),
            &ts_str,
        );
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(audit_uuid)
            .bind(actor_id)
            .bind("BATCH_REGISTER_BATTERIES")
            .bind(&bpan_csv)
            .bind(HashChainService::genesis_hash())
            .bind(&entry_hash)
            .bind(now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!(
            "Batch registered {} batteries for manufacturer {}",
            results.len(),
            manufacturer_code
        );

        Ok(BatchRegistrationResponse {
            manufacturer_id,
            total: results.len(),
            batteries: results,
            audit_id: audit_uuid,
        })
    }

    /// Get batteries belonging to a manufacturer.
    #[instrument(name = "list_manufacturer_batteries", skip(self))]
    pub async fn list_manufacturer_batteries(
        &self,
        manufacturer_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> BpaResult<Vec<ManufacturerBatterySummary>> {
        let rows = sqlx::query(
            r#"SELECT b.bpan, b.production_year, b.battery_category, b.compliance_class,
                      bd.chemistry_type, bd.rated_capacity_kwh, bd.nominal_voltage,
                      bh.state_of_health, bh.total_cycles,
                      brl.registration_status
               FROM batteries b
               JOIN battery_descriptor bd ON b.bpan = bd.bpan
               JOIN battery_health bh ON b.bpan = bh.bpan
               JOIN battery_registration_log brl ON b.bpan = brl.bpan
               WHERE b.manufacturer_id = $1
               ORDER BY b.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(manufacturer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| ManufacturerBatterySummary {
                bpan: r.get("bpan"),
                chemistry_type: r.get("chemistry_type"),
                battery_category: r.get("battery_category"),
                rated_capacity_kwh: r.get("rated_capacity_kwh"),
                nominal_voltage: r.get("nominal_voltage"),
                state_of_health: r.get("state_of_health"),
                total_cycles: r.get("total_cycles"),
                registration_status: r.get("registration_status"),
                production_year: r.get("production_year"),
            })
            .collect())
    }

    /// Get manufacturer dashboard aggregates.
    #[instrument(name = "manufacturer_dashboard", skip(self))]
    pub async fn get_dashboard(&self, manufacturer_id: Uuid) -> BpaResult<ManufacturerDashboard> {
        // Total batteries
        let total_row = sqlx::query(
            "SELECT COUNT(*) as cnt FROM batteries WHERE manufacturer_id = $1",
        )
        .bind(manufacturer_id)
        .fetch_one(&self.pool)
        .await?;
        let total_batteries: i64 = total_row.get("cnt");

        // Batteries by lifecycle state
        let states_row = sqlx::query(
            r#"SELECT
                 COUNT(*) FILTER (WHERE registration_status = 'APPROVED') as operational,
                 COUNT(*) FILTER (WHERE registration_status = 'PENDING') as pending,
                 COUNT(*) FILTER (WHERE registration_status = 'REJECTED') as rejected
               FROM battery_registration_log brl
               JOIN batteries b ON b.bpan = brl.bpan
               WHERE b.manufacturer_id = $1"#,
        )
        .bind(manufacturer_id)
        .fetch_one(&self.pool)
        .await?;
        let operational: i64 = states_row.get("operational");
        let pending: i64 = states_row.get("pending");
        let rejected: i64 = states_row.get("rejected");

        // Average SoH
        let soh_row = sqlx::query(
            r#"SELECT AVG(bh.state_of_health) as avg_soh
               FROM battery_health bh
               JOIN batteries b ON b.bpan = bh.bpan
               WHERE b.manufacturer_id = $1"#,
        )
        .bind(manufacturer_id)
        .fetch_one(&self.pool)
        .await?;
        let avg_soh: Option<f64> = soh_row.get("avg_soh");

        // Compliance violation count
        let violations_row = sqlx::query(
            r#"SELECT COUNT(*) as cnt
               FROM compliance_violation_log cvl
               JOIN batteries b ON b.bpan = cvl.bpan
               WHERE b.manufacturer_id = $1"#,
        )
        .bind(manufacturer_id)
        .fetch_one(&self.pool)
        .await?;
        let violation_count: i64 = violations_row.get("cnt");

        // Second life and EOL counts
        let eol_row = sqlx::query(
            r#"SELECT COUNT(*) as cnt FROM battery_health bh
               JOIN batteries b ON b.bpan = bh.bpan
               WHERE b.manufacturer_id = $1 AND bh.end_of_life = true"#,
        )
        .bind(manufacturer_id)
        .fetch_one(&self.pool)
        .await?;
        let eol_count: i64 = eol_row.get("cnt");

        let second_life_row = sqlx::query(
            r#"SELECT COUNT(*) as cnt FROM reuse_certifications rc
               JOIN batteries b ON b.bpan = rc.bpan
               WHERE b.manufacturer_id = $1"#,
        )
        .bind(manufacturer_id)
        .fetch_one(&self.pool)
        .await?;
        let second_life_count: i64 = second_life_row.get("cnt");

        Ok(ManufacturerDashboard {
            total_batteries,
            operational,
            pending_registrations: pending,
            rejected_registrations: rejected,
            second_life: second_life_count,
            end_of_life: eol_count,
            average_soh: avg_soh.unwrap_or(0.0),
            compliance_violations: violation_count,
        })
    }

    // --- Private helpers ---

    /// Assign a unique 3-char uppercase manufacturer code.
    /// Generates sequentially: AAA, AAB, AAC, ... ZZZ.
    async fn assign_manufacturer_code(&self) -> BpaResult<String> {
        let row = sqlx::query(
            "SELECT manufacturer_code FROM manufacturers ORDER BY manufacturer_code DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        let next = match row {
            Some(r) => {
                let last_code: String = r.get("manufacturer_code");
                Self::increment_code(&last_code)?
            }
            None => "AAA".to_string(),
        };

        // Ensure the code is unique (safety check)
        let exists = sqlx::query("SELECT id FROM manufacturers WHERE manufacturer_code = $1")
            .bind(&next)
            .fetch_optional(&self.pool)
            .await?;
        if exists.is_some() {
            return Err(BpaError::Conflict(format!(
                "manufacturer code '{}' already assigned",
                next
            )));
        }

        Ok(next)
    }

    fn increment_code(code: &str) -> BpaResult<String> {
        if code.len() != 3 {
            return Err(BpaError::Internal("invalid code length".into()));
        }
        let bytes = code.as_bytes();
        let mut c3 = bytes[2];
        let mut c2 = bytes[1];
        let mut c1 = bytes[0];

        c3 += 1;
        if c3 > b'Z' {
            c3 = b'A';
            c2 += 1;
        }
        if c2 > b'Z' {
            c2 = b'A';
            c1 += 1;
        }
        if c1 > b'Z' {
            return Err(BpaError::Internal("manufacturer code space exhausted".into()));
        }

        Ok(format!(
            "{}{}{}",
            c1 as char, c2 as char, c3 as char
        ))
    }

    fn map_chemistry_to_code_inline(chemistry: &str) -> BpaResult<String> {
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

    fn map_category_to_code_inline(category: &str) -> BpaResult<String> {
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

/// Summary of a battery owned by a manufacturer.
#[derive(Debug, Clone)]
pub struct ManufacturerBatterySummary {
    pub bpan: String,
    pub chemistry_type: String,
    pub battery_category: String,
    pub rated_capacity_kwh: f64,
    pub nominal_voltage: f64,
    pub state_of_health: f64,
    pub total_cycles: i32,
    pub registration_status: String,
    pub production_year: i32,
}

/// Manufacturer dashboard aggregates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManufacturerDashboard {
    pub total_batteries: i64,
    pub operational: i64,
    pub pending_registrations: i64,
    pub rejected_registrations: i64,
    pub second_life: i64,
    pub end_of_life: i64,
    pub average_soh: f64,
    pub compliance_violations: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_code() {
        assert_eq!(ManufacturerService::increment_code("AAA").unwrap(), "AAB");
        assert_eq!(ManufacturerService::increment_code("AAZ").unwrap(), "ABA");
        assert_eq!(ManufacturerService::increment_code("AZZ").unwrap(), "BAA");
        assert!(ManufacturerService::increment_code("ZZZ").is_err());
    }

    #[test]
    fn test_increment_code_mid_range() {
        assert_eq!(ManufacturerService::increment_code("TAT").unwrap(), "TAU");
        assert_eq!(ManufacturerService::increment_code("ABC").unwrap(), "ABD");
    }
}
