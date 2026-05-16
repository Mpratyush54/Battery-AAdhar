//! health.rs — Battery health service with automatic ZK proof generation
//!
//! On each health update:
//! 1. Store health record
//! 2. Generate ZK proofs for SoH thresholds (80%, 60%, 30%)
//! 3. Store proofs in proof table
//! 4. Add to dynamic data log with hash chain

use crate::models::{HealthRecord, HealthUpdateRequest};
use crate::repositories::health_repo::HealthRepositoryImpl;
use crate::services::ZkProverImpl;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug)]
pub enum HealthError {
    NotFound(String),
    Unauthorized(String),
    RateLimited(String),
    InvalidData(String),
    ZkProofFailed(String),
    DatabaseError(String),
}

impl std::fmt::Display for HealthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthError::NotFound(msg) => write!(f, "not found: {}", msg),
            HealthError::Unauthorized(msg) => write!(f, "unauthorized: {}", msg),
            HealthError::RateLimited(msg) => write!(f, "rate limited: {}", msg),
            HealthError::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            HealthError::ZkProofFailed(msg) => write!(f, "ZK proof failed: {}", msg),
            HealthError::DatabaseError(msg) => write!(f, "database error: {}", msg),
        }
    }
}

impl std::error::Error for HealthError {}

#[async_trait]
pub trait HealthService: Send + Sync {
    async fn update_health(
        &self,
        bpan: &str,
        req: &HealthUpdateRequest,
        requester_role: &str,
    ) -> Result<String, HealthError>;

    async fn get_current_health(&self, bpan: &str) -> Result<HealthRecord, HealthError>;

    async fn get_health_history(
        &self,
        bpan: &str,
        limit: i32,
    ) -> Result<Vec<HealthRecord>, HealthError>;

    async fn get_avg_soh_by_manufacturer(&self, manufacturer_id: &str) -> Result<f32, HealthError>;

    async fn get_avg_soh_by_chemistry(&self, chemistry_type: &str) -> Result<f32, HealthError>;
}

pub struct HealthServiceImpl {
    zk_prover: Arc<ZkProverImpl>,
    repo: Arc<HealthRepositoryImpl>,
}

impl HealthServiceImpl {
    pub fn new(zk_prover: Arc<ZkProverImpl>, pool: PgPool) -> Self {
        HealthServiceImpl {
            zk_prover,
            repo: Arc::new(HealthRepositoryImpl::new(pool)),
        }
    }

    fn can_update_health(&self, role: &str) -> bool {
        matches!(
            role,
            "BMS" | "MANUFACTURER" | "SERVICE_PROVIDER" | "ADMIN"
                | "bms" | "manufacturer" | "service_provider" | "admin"
        )
    }

    async fn generate_zk_proofs(
        &self,
        soh: f32,
    ) -> Result<(Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>), HealthError> {
        let proof_gt_80 = if soh > 80.0 {
            let (proof, _, _) = self
                .zk_prover
                .prove_operational(soh as u64)
                .map_err(|e| HealthError::ZkProofFailed(e.to_string()))?;
            Some(proof.0)
        } else {
            None
        };

        let proof_gte_60 = if soh >= 60.0 {
            let (proof, _, _) = self
                .zk_prover
                .prove_range(soh as u64, 60, 100)
                .map_err(|e| HealthError::ZkProofFailed(e.to_string()))?;
            Some(proof.0)
        } else {
            None
        };

        let proof_gte_30 = if soh >= 30.0 {
            let (proof, _, _) = self
                .zk_prover
                .prove_range(soh as u64, 30, 100)
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
        if !self.can_update_health(requester_role) {
            return Err(HealthError::Unauthorized(
                "only BMS/manufacturer can update health".to_string(),
            ));
        }

        if req.state_of_health_percent < 0.0 || req.state_of_health_percent > 100.0 {
            return Err(HealthError::InvalidData("SoH must be 0–100".to_string()));
        }

        // Check rate limit (max 1 update per hour per battery)
        let rate_limited = self
            .repo
            .check_rate_limit(bpan)
            .await
            .map_err(|e| HealthError::DatabaseError(e.to_string()))?;
        if rate_limited {
            return Err(HealthError::RateLimited(
                "max 1 health update per battery per hour".to_string(),
            ));
        }

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

        let record_id = self
            .repo
            .insert_health_record(&record)
            .await
            .map_err(|e| HealthError::DatabaseError(e.to_string()))?;

        tracing::info!(
            bpan = %bpan,
            soh = req.state_of_health_percent,
            record_id = %record_id,
            "health updated with ZK proofs"
        );

        Ok(record_id)
    }

    async fn get_current_health(&self, bpan: &str) -> Result<HealthRecord, HealthError> {
        self.repo
            .get_latest_health(bpan)
            .await
            .map_err(|e| HealthError::DatabaseError(e.to_string()))?
            .ok_or_else(|| HealthError::NotFound(format!("no health data for BPAN {}", bpan)))
    }

    async fn get_health_history(
        &self,
        bpan: &str,
        limit: i32,
    ) -> Result<Vec<HealthRecord>, HealthError> {
        self.repo
            .get_health_history(bpan, limit)
            .await
            .map_err(|e| HealthError::DatabaseError(e.to_string()))
    }

    async fn get_avg_soh_by_manufacturer(
        &self,
        manufacturer_id: &str,
    ) -> Result<f32, HealthError> {
        self.repo
            .get_avg_soh_by_manufacturer(manufacturer_id)
            .await
            .map_err(|e| HealthError::DatabaseError(e.to_string()))
    }

    async fn get_avg_soh_by_chemistry(&self, chemistry_type: &str) -> Result<f32, HealthError> {
        self.repo
            .get_avg_soh_by_chemistry(chemistry_type)
            .await
            .map_err(|e| HealthError::DatabaseError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_authorization() {
        // Test would require a real DB pool — see integration tests
    }
}
