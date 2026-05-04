//! health.rs — Battery health service with automatic ZK proof generation
//!
//! On each health update:
//! 1. Store health record
//! 2. Generate ZK proofs for SoH thresholds (80%, 60%, 30%)
//! 3. Store proofs in proof table
//! 4. Add to dynamic data log with hash chain

use crate::models::{HealthRecord, HealthStatus, HealthUpdateRequest};
use crate::services::ZkProverImpl;
use std::sync::Arc;
use async_trait::async_trait;

#[derive(Debug)]
pub enum HealthError {
    NotFound(String),
    Unauthorized(String),
    RateLimited(String),
    InvalidData(String),
    ZkProofFailed(String),
}

impl std::fmt::Display for HealthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthError::NotFound(msg) => write!(f, "not found: {}", msg),
            HealthError::Unauthorized(msg) => write!(f, "unauthorized: {}", msg),
            HealthError::RateLimited(msg) => write!(f, "rate limited: {}", msg),
            HealthError::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            HealthError::ZkProofFailed(msg) => write!(f, "ZK proof failed: {}", msg),
        }
    }
}

impl std::error::Error for HealthError {}

#[async_trait]
pub trait HealthService: Send + Sync {
    /// Update battery health (auto-generates ZK proofs)
    async fn update_health(
        &self,
        bpan: &str,
        req: &HealthUpdateRequest,
        requester_role: &str,
    ) -> Result<String, HealthError>; // Returns record ID

    /// Get current health status
    async fn get_current_health(
        &self,
        bpan: &str,
    ) -> Result<HealthRecord, HealthError>;

    /// Get health history (time-series)
    async fn get_health_history(
        &self,
        bpan: &str,
        limit: i32,
    ) -> Result<Vec<HealthRecord>, HealthError>;

    /// Get average SoH by manufacturer
    async fn get_avg_soh_by_manufacturer(
        &self,
        manufacturer_id: &str,
    ) -> Result<f32, HealthError>;

    /// Get average SoH by chemistry type
    async fn get_avg_soh_by_chemistry(
        &self,
        chemistry_type: &str,
    ) -> Result<f32, HealthError>;
}

pub struct HealthServiceImpl {
    zk_prover: Arc<ZkProverImpl>,
    // TODO Day 7: add repository + rate limiter
}

impl HealthServiceImpl {
    pub fn new(zk_prover: Arc<ZkProverImpl>) -> Self {
        HealthServiceImpl { zk_prover }
    }

    fn can_update_health(&self, role: &str) -> bool {
        matches!(role, "bms" | "manufacturer" | "service_provider" | "admin")
    }

    /// Generate ZK proofs for all SoH thresholds
    async fn generate_zk_proofs(
        &self,
        soh: f32,
    ) -> Result<(Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>), HealthError> {
        // Proof 1: SoH > 80% (Operational)
        let proof_gt_80 = if soh > 80.0 {
            let (proof, _, _) = self.zk_prover.prove_operational(soh as u64)
                .map_err(|e| HealthError::ZkProofFailed(e.to_string()))?;
            Some(proof.0)
        } else {
            None
        };

        // Proof 2: SoH >= 60% (Second Life)
        let proof_gte_60 = if soh >= 60.0 {
            let (proof, _, _) = self.zk_prover.prove_second_life(soh as u64)
                .map_err(|e| HealthError::ZkProofFailed(e.to_string()))?;
            Some(proof.0)
        } else {
            None
        };

        // Proof 3: SoH >= 30% (EOL Process)
        let proof_gte_30 = if soh >= 30.0 {
            let (proof, _, _) = self.zk_prover.prove_range(soh as u64, 30, 100)
                .map_err(|e| HealthError::ZkProofFailed(e.to_string()))?;
            Some(proof.0)
        } else {
            None
        };

        Ok((proof_gt_80, proof_gte_60, proof_gte_30))
    }
}

#[async_trait]
impl HealthService for HealthServiceImpl {
    async fn update_health(
        &self,
        bpan: &str,
        req: &HealthUpdateRequest,
        requester_role: &str,
    ) -> Result<String, HealthError> {
        // Check authorization
        if !self.can_update_health(requester_role) {
            return Err(HealthError::Unauthorized(
                "only BMS/manufacturer can update health".to_string(),
            ));
        }

        // Validate SoH
        if req.state_of_health_percent < 0.0 || req.state_of_health_percent > 100.0 {
            return Err(HealthError::InvalidData("SoH must be 0–100".to_string()));
        }

        // TODO Day 7: Check rate limit (max 1 update per hour per battery)
        // For now, skip rate limit check

        // Create health record
        let mut record = HealthRecord::new(
            bpan.to_string(),
            req.state_of_health_percent,
            req.cycle_count,
            req.degradation_class.clone(),
            requester_role.to_string(),
        );

        // Update optional fields
        if let Some(min_temp) = req.min_temperature_celsius {
            record.min_temperature_celsius = min_temp;
        }
        if let Some(max_temp) = req.max_temperature_celsius {
            record.max_temperature_celsius = max_temp;
        }
        if let Some(avg_temp) = req.average_temperature_celsius {
            record.average_temperature_celsius = avg_temp;
        }
        if let Some(min_voltage) = req.cell_voltage_min_mv {
            record.cell_voltage_min_mv = min_voltage;
        }
        if let Some(max_voltage) = req.cell_voltage_max_mv {
            record.cell_voltage_max_mv = max_voltage;
        }
        if let Some(resistance) = req.internal_resistance_mohm {
            record.internal_resistance_mohm = resistance;
        }
        if let Some(errors) = &req.error_flags {
            record.error_flags = errors.clone();
            record.is_healthy = errors.is_empty();
        }

        // Generate ZK proofs automatically
        let (proof_gt_80, proof_gte_60, proof_gte_30) = 
            self.generate_zk_proofs(req.state_of_health_percent).await?;

        record.zk_proof_soh_gt_80 = proof_gt_80;
        record.zk_proof_soh_gte_60 = proof_gte_60;
        record.zk_proof_soh_gte_30 = proof_gte_30;
        record.proofs_generated_at = Some(chrono::Utc::now());

        let record_id = record.id.to_string();

        // TODO Day 7: Store in DB + dynamic data log with hash chain

        tracing::info!(
            "health updated: {} → SoH={}% status={:?} proofs_generated={}",
            bpan,
            req.state_of_health_percent,
            record.health_status,
            record.zk_proof_soh_gt_80.is_some()
        );

        Ok(record_id)
    }

    async fn get_current_health(
        &self,
        bpan: &str,
    ) -> Result<HealthRecord, HealthError> {
        // TODO Day 7: Fetch latest from DB
        // For now, return mock data

        Ok(HealthRecord::new(
            bpan.to_string(),
            85.5,
            250000,
            "normal".to_string(),
            "bms-001".to_string(),
        ))
    }

    async fn get_health_history(
        &self,
        bpan: &str,
        limit: i32,
    ) -> Result<Vec<HealthRecord>, HealthError> {
        // TODO Day 7: Fetch from DB ordered by date DESC

        Ok(vec![
            HealthRecord::new(
                bpan.to_string(),
                85.5,
                250000,
                "normal".to_string(),
                "bms-001".to_string(),
            ),
            HealthRecord::new(
                bpan.to_string(),
                87.0,
                245000,
                "normal".to_string(),
                "bms-001".to_string(),
            ),
        ])
    }

    async fn get_avg_soh_by_manufacturer(
        &self,
        manufacturer_id: &str,
    ) -> Result<f32, HealthError> {
        // TODO Day 7: Query aggregated SoH by manufacturer
        Ok(80.5)
    }

    async fn get_avg_soh_by_chemistry(
        &self,
        chemistry_type: &str,
    ) -> Result<f32, HealthError> {
        // TODO Day 7: Query aggregated SoH by chemistry
        Ok(81.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_update_health_authorized() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = HealthServiceImpl::new(zk_prover);

        let req = HealthUpdateRequest {
            state_of_health_percent: 85.0,
            cycle_count: 250000,
            degradation_class: "normal".to_string(),
            min_temperature_celsius: Some(15.0),
            max_temperature_celsius: Some(45.0),
            average_temperature_celsius: Some(30.0),
            cell_voltage_min_mv: Some(2500.0),
            cell_voltage_max_mv: Some(4200.0),
            internal_resistance_mohm: Some(15.0),
            error_flags: None,
        };

        // BMS can update
        let result = service
            .update_health("MY008A6FKKKLC1DH80001", &req, "bms")
            .await;
        assert!(result.is_ok());

        // Manufacturer can update
        let result = service
            .update_health("MY008A6FKKKLC1DH80001", &req, "manufacturer")
            .await;
        assert!(result.is_ok());

        // Consumer cannot update
        let result = service
            .update_health("MY008A6FKKKLC1DH80001", &req, "consumer")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_soh() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = HealthServiceImpl::new(zk_prover);

        let req = HealthUpdateRequest {
            state_of_health_percent: 150.0, // Invalid!
            cycle_count: 250000,
            degradation_class: "normal".to_string(),
            min_temperature_celsius: None,
            max_temperature_celsius: None,
            average_temperature_celsius: None,
            cell_voltage_min_mv: None,
            cell_voltage_max_mv: None,
            internal_resistance_mohm: None,
            error_flags: None,
        };

        let result = service
            .update_health("MY008A6FKKKLC1DH80001", &req, "bms")
            .await;

        assert!(result.is_err());
        match result {
            Err(HealthError::InvalidData(_)) => (),
            _ => panic!("expected InvalidData"),
        }
    }
}
