//! carbon.rs — Carbon footprint service with verification
//!
//! Handles submission, verification, and integrity checks.

use crate::models::{CarbonFootprint, CarbonFootprintPublic, CarbonFootprintRequest};

use async_trait::async_trait;

#[derive(Debug)]
pub enum CarbonError {
    NotFound(String),
    Unauthorized(String),
    VerificationFailed(String),
    TamperDetected(String),
    ValidationError(String),
}

impl std::fmt::Display for CarbonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CarbonError::NotFound(msg) => write!(f, "not found: {}", msg),
            CarbonError::Unauthorized(msg) => write!(f, "unauthorized: {}", msg),
            CarbonError::VerificationFailed(msg) => write!(f, "verification failed: {}", msg),
            CarbonError::TamperDetected(msg) => write!(f, "tamper detected: {}", msg),
            CarbonError::ValidationError(msg) => write!(f, "validation error: {}", msg),
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
}

#[derive(Debug)]
pub enum CarbonFootprintOrPublic {
    Full(Box<CarbonFootprint>),
    Public(CarbonFootprintPublic),
}

pub struct CarbonServiceImpl;

impl Default for CarbonServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl CarbonServiceImpl {
    pub fn new() -> Self {
        CarbonServiceImpl
    }

    fn can_submit(&self, role: &str) -> bool {
        matches!(role, "manufacturer" | "importer" | "admin")
    }

    fn can_verify(&self, role: &str) -> bool {
        matches!(role, "verifier" | "regulator" | "admin")
    }

    fn can_see_full(&self, role: &str) -> bool {
        matches!(
            role,
            "manufacturer" | "importer" | "verifier" | "regulator" | "admin"
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
    ) -> Result<String, CarbonError> {
        if !self.can_submit(requester_role) {
            return Err(CarbonError::Unauthorized(
                "only manufacturer can submit carbon data".to_string(),
            ));
        }

        if bpan.len() != 21 {
            return Err(CarbonError::ValidationError("invalid BPAN".to_string()));
        }

        // Validate emissions are non-negative (except recycling avoided mining)
        if data.raw_material_emissions_kg_co2e < 0.0
            || data.manufacturing_emissions_kg_co2e < 0.0
            || data.transport_emissions_kg_co2e < 0.0
            || data.usage_emissions_kg_co2e < 0.0
        {
            return Err(CarbonError::ValidationError(
                "emissions must be non-negative".to_string(),
            ));
        }

        // TODO Day 7: Store in DB, encrypt sensitive fields
        let submission_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("carbon footprint submitted: {}", submission_id);

        Ok(submission_id)
    }

    async fn verify_carbon_footprint(
        &self,
        bpan: &str,
        _verified_by: &str,
        standard: &str,
        requester_role: &str,
    ) -> Result<(), CarbonError> {
        if !self.can_verify(requester_role) {
            return Err(CarbonError::Unauthorized(
                "only verifier can verify carbon data".to_string(),
            ));
        }

        // TODO Day 7: Fetch from DB, check hash integrity
        // If hash doesn't match, raise TamperDetected error
        // Otherwise, mark verified=true, set verified_by and verified_at

        tracing::info!("carbon footprint verified for BPAN {}: {}", bpan, standard);

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

        // TODO Day 7: Fetch from DB
        // For now, return mock data

        let cf = CarbonFootprint {
            bpan: bpan.to_string(),
            raw_material_emissions_kg_co2e: 45.0,
            raw_material_source_country: "Indonesia".to_string(),
            mining_method: "Brine Evaporation".to_string(),
            manufacturing_emissions_kg_co2e: 35.0,
            manufacturing_location: "China".to_string(),
            factory_energy_source: "Renewable".to_string(),
            cell_production_method: "Wet Coating".to_string(),
            transport_emissions_kg_co2e: 12.0,
            transport_distance_km: 15000.0,
            transport_mode: "Sea".to_string(),
            transport_packaging: "Recyclable".to_string(),
            usage_emissions_kg_co2e: 80.0,
            usage_years: 8,
            usage_grid_emissions_factor: 500.0,
            usage_annual_km: 15000,
            recycling_emissions_kg_co2e: -15.0,
            recycling_recovery_rate: 85.0,
            recycling_avoided_mining: 30.0,
            recycling_method: "Hydrometallurgical".to_string(),
            total_emissions_kg_co2e: 157.0,
            emissions_per_kwh: 5.23,
            carbon_hash: "abc123def456".to_string(),
            submitted_by: "mfr-001".to_string(),
            submitted_at: chrono::Utc::now(),
            submitted_version: 1,
            verified: true,
            verified_by: Some("TUV-INDIA".to_string()),
            verified_at: Some(chrono::Utc::now()),
            verification_standard: Some("ISO 14040".to_string()),
        };

        if self.can_see_full(requester_role) {
            Ok(CarbonFootprintOrPublic::Full(Box::new(cf)))
        } else {
            Ok(CarbonFootprintOrPublic::Public((&cf).into()))
        }
    }

    async fn check_tamper(&self, _bpan: &str) -> Result<bool, CarbonError> {
        // TODO Day 7: Fetch from DB, recompute hash
        // If stored_hash != computed_hash, return true (tamper detected)
        // Otherwise false

        // For now, return false (no tamper)
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_submit_unauthorized() {
        let service = CarbonServiceImpl::new();
        let req = CarbonFootprintRequest {
            raw_material_emissions_kg_co2e: 45.0,
            raw_material_source_country: "Indonesia".to_string(),
            mining_method: "Brine".to_string(),
            manufacturing_emissions_kg_co2e: 35.0,
            manufacturing_location: "China".to_string(),
            factory_energy_source: "Renewable".to_string(),
            cell_production_method: "Wet".to_string(),
            transport_emissions_kg_co2e: 12.0,
            transport_distance_km: 15000.0,
            transport_mode: "Sea".to_string(),
            transport_packaging: "Cardboard".to_string(),
            usage_emissions_kg_co2e: 80.0,
            usage_years: 8,
            usage_grid_emissions_factor: 500.0,
            usage_annual_km: 15000,
            recycling_emissions_kg_co2e: -15.0,
            recycling_recovery_rate: 85.0,
            recycling_avoided_mining: 30.0,
            recycling_method: "Hydrometallurgical".to_string(),
        };

        let result = service
            .submit_carbon_footprint("MY008A6FKKKLC1DH80001", &req, "consumer")
            .await;

        assert!(result.is_err());
        match result {
            Err(CarbonError::Unauthorized(_)) => (),
            _ => panic!("expected Unauthorized"),
        }
    }

    #[tokio::test]
    async fn test_verify_authorization() {
        let service = CarbonServiceImpl::new();

        // Consumer cannot verify
        let result = service
            .verify_carbon_footprint("MY008A6FKKKLC1DH80001", "consumer", "ISO 14040", "consumer")
            .await;

        assert!(result.is_err());

        // Verifier can verify
        let result = service
            .verify_carbon_footprint(
                "MY008A6FKKKLC1DH80001",
                "TUV-INDIA",
                "ISO 14040",
                "verifier",
            )
            .await;

        assert!(result.is_ok());
    }
}
