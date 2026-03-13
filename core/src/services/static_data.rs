use chrono::Utc;
use sqlx::{Pool, Postgres};
use tracing::{info, error, instrument};
use uuid::Uuid;

use crate::errors::{BpaError, BpaResult};
use crate::services::encryption::EncryptionService;
use crate::services::hash_chain::HashChainService;

/// Manages static data submission and updates for registered batteries.
/// Static data is the Layer 1/2 data that does not change during normal operation
/// (chemistry, capacity, material composition, etc.).
///
/// Per BPA guidelines:
/// - Static data is uploaded by the manufacturer/importer after BPAN registration
/// - Updates require audit logging and hash recalculation
/// - Signatures are recorded for tamper evidence
#[derive(Clone)]
pub struct StaticDataService {
    pool: Pool<Postgres>,
    encryption: EncryptionService,
}

impl StaticDataService {
    pub fn new(pool: Pool<Postgres>, encryption: EncryptionService) -> Self {
        Self { pool, encryption }
    }

    /// Submit material composition data for a battery (Phase 2).
    #[instrument(name = "submit_material_composition", skip(self, request))]
    pub async fn submit_material_composition(
        &self,
        request: MaterialCompositionRequest,
        actor_id: Uuid,
    ) -> BpaResult<Uuid> {
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();

        // Encrypt detailed composition info
        let encrypted_details = self.encryption.encrypt(&request.detailed_composition)?;

        let mut tx = self.pool.begin().await?;

        let query = "INSERT INTO battery_material_composition (id, bpan, cathode_material, anode_material, electrolyte_type, separator_material, lithium_content_g, cobalt_content_g, nickel_content_g, recyclable_percentage, encrypted_details, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)";
        sqlx::query(query)
            .bind(&id)
            .bind(&request.bpan)
            .bind(&request.cathode_material)
            .bind(&request.anode_material)
            .bind(&request.electrolyte_type)
            .bind(&request.separator_material)
            .bind(request.lithium_content_g)
            .bind(request.cobalt_content_g)
            .bind(request.nickel_content_g)
            .bind(request.recyclable_percentage)
            .bind(&encrypted_details)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert material composition: {:?}", e);
                BpaError::Database(e)
            })?;

        // Log the submission
        let log_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let event_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "SUBMIT_MATERIAL_COMPOSITION",
            &request.bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        let log_query = "INSERT INTO static_data_submission_log (id, bpan, submitted_by, data_section, data_hash, previous_event_hash, event_hash, submitted_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        sqlx::query(log_query)
            .bind(&log_id)
            .bind(&request.bpan)
            .bind(&actor_id)
            .bind("MATERIAL_COMPOSITION")
            .bind(&event_hash)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert static_data_submission_log: {:?}", e);
                BpaError::Database(e)
            })?;

        // Audit log
        let audit_id = Uuid::new_v4();
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&actor_id)
            .bind("SUBMIT_MATERIAL_COMPOSITION")
            .bind(&request.bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Material composition submitted for BPAN: {}", request.bpan);
        Ok(id)
    }

    /// Update static data for a battery (requires logging what changed).
    #[instrument(name = "update_static_data", skip(self))]
    pub async fn update_battery_descriptor(
        &self,
        bpan: &str,
        updates: DescriptorUpdateRequest,
        actor_id: Uuid,
    ) -> BpaResult<()> {
        let now = Utc::now().naive_utc();

        let mut tx = self.pool.begin().await?;

        // Fetch current to log previous state
        let current: Option<(f64, f64, f64, f64, String)> = sqlx::query_as(
            "SELECT nominal_voltage, rated_capacity_kwh, energy_density, weight_kg, form_factor FROM battery_descriptor WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_optional(&mut *tx)
        .await?;

        let current = current.ok_or_else(|| {
            BpaError::NotFound(format!("No battery descriptor found for BPAN: {}", bpan))
        })?;

        // Build update query dynamically based on provided fields
        let new_voltage = updates.nominal_voltage.unwrap_or(current.0);
        let new_capacity = updates.rated_capacity_kwh.unwrap_or(current.1);
        let new_density = updates.energy_density.unwrap_or(current.2);
        let new_weight = updates.weight_kg.unwrap_or(current.3);
        let new_form = updates.form_factor.as_deref().unwrap_or(&current.4);

        let update_query = "UPDATE battery_descriptor SET nominal_voltage = $1, rated_capacity_kwh = $2, energy_density = $3, weight_kg = $4, form_factor = $5 WHERE bpan = $6";
        sqlx::query(update_query)
            .bind(new_voltage)
            .bind(new_capacity)
            .bind(new_density)
            .bind(new_weight)
            .bind(new_form)
            .bind(bpan)
            .execute(&mut *tx)
            .await?;

        // Recompute static hash and update batteries table
        let chemistry: Option<(String,)> = sqlx::query_as(
            "SELECT chemistry_type FROM battery_descriptor WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((chem,)) = chemistry {
            let new_static_hash = HashChainService::compute_static_hash(
                bpan, &chem, new_voltage, new_capacity, new_form,
            );
            sqlx::query("UPDATE batteries SET static_hash = $1 WHERE bpan = $2")
                .bind(&new_static_hash)
                .bind(bpan)
                .execute(&mut *tx)
                .await?;
        }

        // Log the update
        let log_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let event_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "UPDATE_DESCRIPTOR",
            bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO static_data_update_log (id, bpan, updated_by, field_name, previous_hash, new_hash, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&log_id)
            .bind(bpan)
            .bind(&actor_id)
            .bind("BATTERY_DESCRIPTOR")
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Audit log
        let audit_id = Uuid::new_v4();
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&actor_id)
            .bind("UPDATE_STATIC_DATA")
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Battery descriptor updated for BPAN: {}", bpan);
        Ok(())
    }

    /// Sign a section of static data (creates a tamper-evident signature record).
    #[instrument(name = "sign_static_data", skip(self))]
    pub async fn sign_static_data(
        &self,
        bpan: &str,
        data_section: &str,
        data_hash: &str,
        signature: &[u8],
        certificate_id: Uuid,
    ) -> BpaResult<Uuid> {
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();

        sqlx::query("INSERT INTO static_signatures (id, bpan, data_section, data_hash, signature, certificate_id, signed_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&id)
            .bind(bpan)
            .bind(data_section)
            .bind(data_hash)
            .bind(signature)
            .bind(&certificate_id)
            .bind(&now)
            .execute(&self.pool)
            .await?;

        info!("Static data signed for BPAN {} section {}", bpan, data_section);
        Ok(id)
    }
}

/// Request to submit material composition data.
#[derive(Debug, Clone)]
pub struct MaterialCompositionRequest {
    pub bpan: String,
    pub cathode_material: String,
    pub anode_material: String,
    pub electrolyte_type: String,
    pub separator_material: String,
    pub lithium_content_g: f64,
    pub cobalt_content_g: f64,
    pub nickel_content_g: f64,
    pub recyclable_percentage: f64,
    pub detailed_composition: String,  // JSON blob with full composition details
}

/// Optional fields for updating a battery descriptor.
#[derive(Debug, Clone, Default)]
pub struct DescriptorUpdateRequest {
    pub nominal_voltage: Option<f64>,
    pub rated_capacity_kwh: Option<f64>,
    pub energy_density: Option<f64>,
    pub weight_kg: Option<f64>,
    pub form_factor: Option<String>,
}
