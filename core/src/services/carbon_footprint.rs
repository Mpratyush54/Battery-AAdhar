use chrono::Utc;
use sqlx::{Pool, Postgres};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::errors::{BpaError, BpaResult};
use crate::services::hash_chain::HashChainService;
use crate::services::validation::ValidationService;

/// Manages carbon footprint data for batteries (Phase 3 of BPA guideline).
///
/// Carbon footprint is calculated across 5 lifecycle stages:
/// 1. Raw material extraction & processing
/// 2. Manufacturing
/// 3. Transportation
/// 4. Usage phase
/// 5. End-of-life recycling
///
/// The total is stored and verified against the carbon_hash in the batteries table.
#[derive(Clone)]
pub struct CarbonFootprintService {
    pool: Pool<Postgres>,
}

impl CarbonFootprintService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Submit carbon footprint data for a battery.
    #[instrument(name = "submit_carbon_footprint", skip(self))]
    pub async fn submit_footprint(
        &self,
        request: CarbonFootprintRequest,
        actor_id: Uuid,
    ) -> BpaResult<Uuid> {
        // Validate all emission values
        ValidationService::validate_emission("Raw material", request.raw_material_emission)?;
        ValidationService::validate_emission("Manufacturing", request.manufacturing_emission)?;
        ValidationService::validate_emission("Transport", request.transport_emission)?;
        ValidationService::validate_emission("Usage", request.usage_emission)?;
        ValidationService::validate_emission("Recycling", request.recycling_emission)?;

        let total = request.raw_material_emission
            + request.manufacturing_emission
            + request.transport_emission
            + request.usage_emission
            + request.recycling_emission;

        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();

        let mut tx = self.pool.begin().await?;

        // Insert carbon footprint record
        sqlx::query("INSERT INTO carbon_footprint (id, bpan, raw_material_emission, manufacturing_emission, transport_emission, usage_emission, recycling_emission, total_emission, verified, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)")
            .bind(&id)
            .bind(&request.bpan)
            .bind(request.raw_material_emission)
            .bind(request.manufacturing_emission)
            .bind(request.transport_emission)
            .bind(request.usage_emission)
            .bind(request.recycling_emission)
            .bind(total)
            .bind(false) // Not verified until an auditor confirms
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Update the carbon_hash in the batteries table
        let carbon_hash = HashChainService::compute_carbon_hash(
            &request.bpan,
            request.raw_material_emission,
            request.manufacturing_emission,
            request.transport_emission,
            request.usage_emission,
            request.recycling_emission,
        );

        sqlx::query("UPDATE batteries SET carbon_hash = $1 WHERE bpan = $2")
            .bind(&carbon_hash)
            .bind(&request.bpan)
            .execute(&mut *tx)
            .await?;

        // Audit log
        let audit_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let entry_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "SUBMIT_CARBON_FOOTPRINT",
            &request.bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&actor_id)
            .bind("SUBMIT_CARBON_FOOTPRINT")
            .bind(&request.bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&entry_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!(
            "Carbon footprint submitted for BPAN {}: {:.2} kg CO2e total",
            request.bpan, total
        );
        Ok(id)
    }

    /// Verify a carbon footprint submission (mark as auditor-verified).
    #[instrument(name = "verify_carbon_footprint", skip(self))]
    pub async fn verify_footprint(
        &self,
        footprint_id: Uuid,
        verifier_id: Uuid,
    ) -> BpaResult<()> {
        let now = Utc::now().naive_utc();

        let result = sqlx::query("UPDATE carbon_footprint SET verified = $1 WHERE id = $2 AND verified = false")
            .bind(true)
            .bind(&footprint_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(BpaError::NotFound(format!(
                "No unverified carbon footprint found with id {}",
                footprint_id
            )));
        }

        // Audit log
        let audit_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let entry_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "VERIFY_CARBON_FOOTPRINT",
            &footprint_id.to_string(),
            &verifier_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&verifier_id)
            .bind("VERIFY_CARBON_FOOTPRINT")
            .bind(&footprint_id.to_string())
            .bind(&HashChainService::genesis_hash())
            .bind(&entry_hash)
            .bind(&now)
            .execute(&self.pool)
            .await?;

        info!("Carbon footprint {} verified by {}", footprint_id, verifier_id);
        Ok(())
    }

    /// Get the latest carbon footprint for a battery.
    pub async fn get_footprint(&self, bpan: &str) -> BpaResult<CarbonFootprintRecord> {
        let row: Option<(Uuid, f64, f64, f64, f64, f64, f64, bool, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT id, raw_material_emission, manufacturing_emission, transport_emission, usage_emission, recycling_emission, total_emission, verified, created_at FROM carbon_footprint WHERE bpan = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|(id, raw, mfg, trans, usage, recy, total, verified, at)| CarbonFootprintRecord {
            id,
            bpan: bpan.to_string(),
            raw_material_emission: raw,
            manufacturing_emission: mfg,
            transport_emission: trans,
            usage_emission: usage,
            recycling_emission: recy,
            total_emission: total,
            verified,
            created_at: at,
        })
        .ok_or_else(|| BpaError::NotFound(format!("No carbon footprint for BPAN: {}", bpan)))
    }

    /// Verify the integrity of a carbon footprint by recomputing its hash
    /// and comparing with the carbon_hash stored in the batteries table.
    pub async fn verify_integrity(&self, bpan: &str) -> BpaResult<bool> {
        let footprint = self.get_footprint(bpan).await?;

        let computed_hash = HashChainService::compute_carbon_hash(
            bpan,
            footprint.raw_material_emission,
            footprint.manufacturing_emission,
            footprint.transport_emission,
            footprint.usage_emission,
            footprint.recycling_emission,
        );

        let stored_hash: Option<(String,)> = sqlx::query_as(
            "SELECT carbon_hash FROM batteries WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await?;

        match stored_hash {
            Some((hash,)) => Ok(computed_hash == hash),
            None => Err(BpaError::NotFound(format!("No battery record for BPAN: {}", bpan))),
        }
    }
}

/// Request to submit carbon footprint data (all values in kg CO2e).
#[derive(Debug, Clone)]
pub struct CarbonFootprintRequest {
    pub bpan: String,
    pub raw_material_emission: f64,
    pub manufacturing_emission: f64,
    pub transport_emission: f64,
    pub usage_emission: f64,
    pub recycling_emission: f64,
}

/// A carbon footprint record.
#[derive(Debug, Clone)]
pub struct CarbonFootprintRecord {
    pub id: Uuid,
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
