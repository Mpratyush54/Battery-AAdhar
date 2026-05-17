//! carbon.rs — Carbon footprint service with verification and DB persistence
//!
//! Handles submission, verification, and integrity checks for BCF data.
//! All operations are backed by PostgreSQL with hash-chain audit trails.

use crate::models::{CarbonFootprint, CarbonFootprintPublic, CarbonFootprintRequest};
use crate::repositories::carbon_repo::CarbonRepository;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug)]
pub enum CarbonError {
    NotFound(String),
    Unauthorized(String),
    VerificationFailed(String),
    TamperDetected(String),
    ValidationError(String),
    DatabaseError(String),
}

impl std::fmt::Display for CarbonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CarbonError::NotFound(msg) => write!(f, "not found: {}", msg),
            CarbonError::Unauthorized(msg) => write!(f, "unauthorized: {}", msg),
            CarbonError::VerificationFailed(msg) => write!(f, "verification failed: {}", msg),
            CarbonError::TamperDetected(msg) => write!(f, "tamper detected: {}", msg),
            CarbonError::ValidationError(msg) => write!(f, "validation error: {}", msg),
            CarbonError::DatabaseError(msg) => write!(f, "database error: {}", msg),
        }
    }
}

impl std::error::Error for CarbonError {}

#[async_trait]
pub trait CarbonService: Send + Sync {
    async fn submit_carbon_footprint(
        &self,
        bpan: &str,
        data: &CarbonFootprintRequest,
        requester_role: &str,
        submitter_id: &str,
    ) -> Result<String, CarbonError>;

    async fn verify_carbon_footprint(
        &self,
        bpan: &str,
        verified_by: &str,
        standard: &str,
        requester_role: &str,
    ) -> Result<(), CarbonError>;

    async fn get_carbon_footprint(
        &self,
        bpan: &str,
        requester_role: &str,
    ) -> Result<CarbonFootprintOrPublic, CarbonError>;

    async fn check_tamper(&self, bpan: &str) -> Result<bool, CarbonError>;

    async fn compare_carbon_footprints(
        &self,
        bpan_a: &str,
        bpan_b: &str,
    ) -> Result<CarbonComparison, CarbonError>;
}

#[derive(Debug)]
pub enum CarbonFootprintOrPublic {
    Full(Box<CarbonFootprint>),
    Public(CarbonFootprintPublic),
}

#[derive(Debug)]
pub struct CarbonComparison {
    pub bpan_a: String,
    pub bpan_b: String,
    pub total_a: f32,
    pub total_b: f32,
    pub delta: f32,
    pub stage_deltas: CarbonStageDeltas,
}

#[derive(Debug)]
pub struct CarbonStageDeltas {
    pub raw_material: f32,
    pub manufacturing: f32,
    pub transport: f32,
    pub usage: f32,
    pub recycling: f32,
}

pub struct CarbonServiceImpl {
    repo: Arc<dyn CarbonRepository>,
}

impl CarbonServiceImpl {
    pub fn new(pool: PgPool) -> Self {
        CarbonServiceImpl {
            repo: Arc::new(crate::repositories::carbon_repo::CarbonRepositoryImpl::new(pool)),
        }
    }

    pub fn new_stub() -> Self {
        CarbonServiceImpl {
            repo: Arc::new(crate::repositories::carbon_repo::CarbonRepositoryImpl::new_stub()),
        }
    }

    fn can_submit(&self, role: &str) -> bool {
        matches!(
            role,
            "MANUFACTURER" | "IMPORTER" | "ADMIN" | "manufacturer" | "importer" | "admin"
        )
    }

    fn can_verify(&self, role: &str) -> bool {
        matches!(
            role,
            "VERIFIER" | "REGULATOR" | "ADMIN" | "verifier" | "regulator" | "admin"
        )
    }

    fn can_see_full(&self, role: &str) -> bool {
        matches!(
            role,
            "MANUFACTURER"
                | "IMPORTER"
                | "VERIFIER"
                | "REGULATOR"
                | "ADMIN"
                | "manufacturer"
                | "importer"
                | "verifier"
                | "regulator"
                | "admin"
        )
    }
}

#[async_trait]
impl CarbonService for CarbonServiceImpl {
    async fn submit_carbon_footprint(
        &self,
        bpan: &str,
        data: &CarbonFootprintRequest,
        requester_role: &str,
        submitter_id: &str,
    ) -> Result<String, CarbonError> {
        if !self.can_submit(requester_role) {
            return Err(CarbonError::Unauthorized(
                "only manufacturer/importer can submit carbon data".to_string(),
            ));
        }

        if bpan.len() != 21 {
            return Err(CarbonError::ValidationError("invalid BPAN".to_string()));
        }

        if data.raw_material_emissions_kg_co2e < 0.0
            || data.manufacturing_emissions_kg_co2e < 0.0
            || data.transport_emissions_kg_co2e < 0.0
            || data.usage_emissions_kg_co2e < 0.0
        {
            return Err(CarbonError::ValidationError(
                "emissions must be non-negative".to_string(),
            ));
        }

        // Compute total emissions
        let total = data.raw_material_emissions_kg_co2e
            + data.manufacturing_emissions_kg_co2e
            + data.transport_emissions_kg_co2e
            + data.usage_emissions_kg_co2e
            + data.recycling_emissions_kg_co2e;

        // Build carbon footprint record
        let cf = CarbonFootprint {
            bpan: bpan.to_string(),
            raw_material_emissions_kg_co2e: data.raw_material_emissions_kg_co2e,
            raw_material_source_country: data.raw_material_source_country.clone(),
            mining_method: data.mining_method.clone(),
            manufacturing_emissions_kg_co2e: data.manufacturing_emissions_kg_co2e,
            manufacturing_location: data.manufacturing_location.clone(),
            factory_energy_source: data.factory_energy_source.clone(),
            cell_production_method: data.cell_production_method.clone(),
            transport_emissions_kg_co2e: data.transport_emissions_kg_co2e,
            transport_distance_km: data.transport_distance_km,
            transport_mode: data.transport_mode.clone(),
            transport_packaging: data.transport_packaging.clone(),
            usage_emissions_kg_co2e: data.usage_emissions_kg_co2e,
            usage_years: data.usage_years,
            usage_grid_emissions_factor: data.usage_grid_emissions_factor,
            usage_annual_km: data.usage_annual_km,
            recycling_emissions_kg_co2e: data.recycling_emissions_kg_co2e,
            recycling_recovery_rate: data.recycling_recovery_rate,
            recycling_avoided_mining: data.recycling_avoided_mining,
            recycling_method: data.recycling_method.clone(),
            total_emissions_kg_co2e: total,
            emissions_per_kwh: 0.0, // Computed later when capacity is known
            carbon_hash: String::new(),               // Computed by repo
            submitted_by: submitter_id.to_string(),
            submitted_at: chrono::Utc::now(),
            submitted_version: 1,
            verified: false,
            verified_by: None,
            verified_at: None,
            verification_standard: None,
        };

        let submission_id = self
            .repo
            .insert_carbon_footprint(&cf)
            .await
            .map_err(|e| CarbonError::DatabaseError(e.to_string()))?;

        tracing::info!(
            bpan = %bpan,
            submission_id = %submission_id,
            total_emissions = total,
            "carbon footprint submitted"
        );

        Ok(submission_id)
    }

    async fn verify_carbon_footprint(
        &self,
        bpan: &str,
        verified_by: &str,
        standard: &str,
        requester_role: &str,
    ) -> Result<(), CarbonError> {
        if !self.can_verify(requester_role) {
            return Err(CarbonError::Unauthorized(
                "only verifier/regulator can verify carbon data".to_string(),
            ));
        }

        // Check tamper before verifying
        let tampered = self.check_tamper(bpan).await?;
        if tampered {
            return Err(CarbonError::TamperDetected(
                "carbon data hash mismatch — possible tampering".to_string(),
            ));
        }

        self.repo
            .verify_carbon_footprint(bpan, verified_by, standard)
            .await
            .map_err(|e| CarbonError::DatabaseError(e.to_string()))?;

        tracing::info!(
            bpan = %bpan,
            verified_by = %verified_by,
            standard = %standard,
            "carbon footprint verified"
        );

        Ok(())
    }

    async fn get_carbon_footprint(
        &self,
        bpan: &str,
        requester_role: &str,
    ) -> Result<CarbonFootprintOrPublic, CarbonError> {
        if bpan.len() != 21 {
            return Err(CarbonError::NotFound("invalid BPAN".to_string()));
        }

        let cf = self
            .repo
            .get_by_bpan(bpan)
            .await
            .map_err(|e| CarbonError::DatabaseError(e.to_string()))?
            .ok_or_else(|| CarbonError::NotFound(format!("no carbon data for BPAN {}", bpan)))?;

        if self.can_see_full(requester_role) {
            Ok(CarbonFootprintOrPublic::Full(Box::new(cf)))
        } else {
            Ok(CarbonFootprintOrPublic::Public((&cf).into()))
        }
    }

    async fn check_tamper(&self, bpan: &str) -> Result<bool, CarbonError> {
        let cf = self
            .repo
            .get_by_bpan(bpan)
            .await
            .map_err(|e| CarbonError::DatabaseError(e.to_string()))?;

        let cf = match cf {
            Some(c) => c,
            None => return Ok(false), // No data = no tamper
        };

        // Recompute hash and compare with stored hash
        let computed_hash = cf.recompute_hash();
        Ok(cf.carbon_hash != computed_hash)
    }

    async fn compare_carbon_footprints(
        &self,
        bpan_a: &str,
        bpan_b: &str,
    ) -> Result<CarbonComparison, CarbonError> {
        let cf_a = self
            .repo
            .get_by_bpan(bpan_a)
            .await
            .map_err(|e| CarbonError::DatabaseError(e.to_string()))?
            .ok_or_else(|| CarbonError::NotFound(format!("no carbon data for BPAN {}", bpan_a)))?;

        let cf_b = self
            .repo
            .get_by_bpan(bpan_b)
            .await
            .map_err(|e| CarbonError::DatabaseError(e.to_string()))?
            .ok_or_else(|| CarbonError::NotFound(format!("no carbon data for BPAN {}", bpan_b)))?;

        Ok(CarbonComparison {
            bpan_a: bpan_a.to_string(),
            bpan_b: bpan_b.to_string(),
            total_a: cf_a.total_emissions_kg_co2e,
            total_b: cf_b.total_emissions_kg_co2e,
            delta: cf_a.total_emissions_kg_co2e - cf_b.total_emissions_kg_co2e,
            stage_deltas: CarbonStageDeltas {
                raw_material: cf_a.raw_material_emissions_kg_co2e
                    - cf_b.raw_material_emissions_kg_co2e,
                manufacturing: cf_a.manufacturing_emissions_kg_co2e
                    - cf_b.manufacturing_emissions_kg_co2e,
                transport: cf_a.transport_emissions_kg_co2e
                    - cf_b.transport_emissions_kg_co2e,
                usage: cf_a.usage_emissions_kg_co2e - cf_b.usage_emissions_kg_co2e,
                recycling: cf_a.recycling_emissions_kg_co2e
                    - cf_b.recycling_emissions_kg_co2e,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_authorization() {
        let service = CarbonServiceImpl::new_stub();

        assert!(service.can_submit("MANUFACTURER"));
        assert!(service.can_submit("manufacturer"));
        assert!(!service.can_submit("consumer"));
        assert!(!service.can_submit("CONSUMER"));

        assert!(service.can_verify("VERIFIER"));
        assert!(service.can_verify("regulator"));
        assert!(!service.can_verify("consumer"));
    }
}
