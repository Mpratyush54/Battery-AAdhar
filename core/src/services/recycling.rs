//! recycling.rs — Battery recycling with material recovery tracking
//!
//! Records: method, weight processed, recovery rates (Li, Co, Ni %)
//! Hash-chained for integrity.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRates {
    pub lithium_percent: f32,  // Li recovery %
    pub cobalt_percent: f32,   // Co recovery %
    pub nickel_percent: f32,   // Ni recovery %
    pub other_percent: f32,    // Other materials
}

impl Default for RecoveryRates {
    fn default() -> Self {
        // Typical recovery rates
        RecoveryRates {
            lithium_percent: 95.0,
            cobalt_percent: 99.0,
            nickel_percent: 94.0,
            other_percent: 85.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecyclingCertification {
    pub id: Uuid,
    pub bpan: String,
    pub recycled_by: String, // Recycler ID
    pub recycled_at: DateTime<Utc>,
    pub recycling_method: String, // "hydrometallurgical", "pyrometallurgical", "mechanical"
    pub weight_processed_kg: f32,
    pub recovery_rates: RecoveryRates,
    pub certifying_standard: String, // "ISO 14040", "R2C2", etc.
    pub certification_hash: String,
}

impl RecyclingCertification {
    pub fn new(
        bpan: String,
        recycled_by: String,
        method: String,
        weight_kg: f32,
        standard: String,
    ) -> Self {
        RecyclingCertification {
            id: Uuid::new_v4(),
            bpan,
            recycled_by,
            recycled_at: Utc::now(),
            recycling_method: method,
            weight_processed_kg: weight_kg,
            recovery_rates: RecoveryRates::default(),
            certifying_standard: standard,
            certification_hash: String::new(),
        }
    }

    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.bpan.as_bytes());
        hasher.update(self.recycled_by.as_bytes());
        hasher.update(self.recycled_at.to_rfc3339().as_bytes());
        hasher.update(self.recycling_method.as_bytes());
        hasher.update(self.weight_processed_kg.to_le_bytes());
        hasher.update(self.recovery_rates.lithium_percent.to_le_bytes());
        hasher.update(self.recovery_rates.cobalt_percent.to_le_bytes());
        hasher.update(self.recovery_rates.nickel_percent.to_le_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn verify_hash_integrity(&self) -> bool {
        self.certification_hash == self.compute_hash()
    }
}

#[derive(Debug)]
pub enum RecyclingError {
    InvalidData(String),
    UnauthorizedRole(String),
    StorageError(String),
}

impl std::fmt::Display for RecyclingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecyclingError::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            RecyclingError::UnauthorizedRole(msg) => write!(f, "unauthorized: {}", msg),
            RecyclingError::StorageError(msg) => write!(f, "storage error: {}", msg),
        }
    }
}

impl std::error::Error for RecyclingError {}

#[async_trait]
pub trait RecyclingService: Send + Sync {
    /// Record battery recycling with recovery rates
    async fn record_recycling(
        &self,
        bpan: &str,
        recycled_by: &str,
        method: &str,
        weight_kg: f32,
        standard: &str,
        recovery_rates: RecoveryRates,
    ) -> Result<String, RecyclingError>; // Returns certification ID
}

pub struct RecyclingServiceImpl {
    repo: std::sync::Arc<dyn crate::repositories::recycling_repo::RecyclingRepository>,
}

impl RecyclingServiceImpl {
    pub fn new(repo: std::sync::Arc<dyn crate::repositories::recycling_repo::RecyclingRepository>) -> Self {
        RecyclingServiceImpl { repo }
    }
}

#[async_trait]
impl RecyclingService for RecyclingServiceImpl {
    async fn record_recycling(
        &self,
        bpan: &str,
        recycled_by: &str,
        method: &str,
        weight_kg: f32,
        standard: &str,
        recovery_rates: RecoveryRates,
    ) -> Result<String, RecyclingError> {
        // Validate recovery rates (must be 0–100%)
        if recovery_rates.lithium_percent < 0.0 || recovery_rates.lithium_percent > 100.0 {
            return Err(RecyclingError::InvalidData("Li recovery must be 0–100%".to_string()));
        }
        if recovery_rates.cobalt_percent < 0.0 || recovery_rates.cobalt_percent > 100.0 {
            return Err(RecyclingError::InvalidData("Co recovery must be 0–100%".to_string()));
        }
        if recovery_rates.nickel_percent < 0.0 || recovery_rates.nickel_percent > 100.0 {
            return Err(RecyclingError::InvalidData("Ni recovery must be 0–100%".to_string()));
        }

        // Create certification
        let mut cert = RecyclingCertification::new(
            bpan.to_string(),
            recycled_by.to_string(),
            method.to_string(),
            weight_kg,
            standard.to_string(),
        );
        cert.recovery_rates = recovery_rates;
        cert.certification_hash = cert.compute_hash();

        // Store in DB via repository
        self.repo.insert_recycling(
            &cert.bpan,
            &cert.recycled_by,
            &cert.recycling_method,
            cert.weight_processed_kg,
            &cert.certifying_standard,
            cert.recovery_rates.lithium_percent,
            cert.recovery_rates.cobalt_percent,
            cert.recovery_rates.nickel_percent,
            &cert.certification_hash,
        ).await.map_err(|e| RecyclingError::StorageError(e.to_string()))?;

        tracing::info!(
            "recycling recorded: {} method={} Li={}% Co={}% Ni={}%",
            bpan,
            method,
            cert.recovery_rates.lithium_percent,
            cert.recovery_rates.cobalt_percent,
            cert.recovery_rates.nickel_percent
        );

        Ok(cert.id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::repositories::battery_repo::RepositoryError;
    use crate::repositories::recycling_repo::CircularEconomyMetrics;

    struct MockRecyclingRepo;

    #[async_trait]
    impl crate::repositories::recycling_repo::RecyclingRepository for MockRecyclingRepo {
        async fn insert_recycling(
            &self,
            _bpan: &str,
            _recycled_by: &str,
            _method: &str,
            _weight_kg: f32,
            _standard: &str,
            _li_percent: f32,
            _co_percent: f32,
            _ni_percent: f32,
            _cert_hash: &str,
        ) -> Result<String, RepositoryError> {
            Ok(uuid::Uuid::new_v4().to_string())
        }

        async fn get_metrics_by_manufacturer(
            &self,
            _manufacturer_id: &str,
        ) -> Result<CircularEconomyMetrics, RepositoryError> {
            unimplemented!()
        }

        async fn get_metrics_by_chemistry(
            &self,
            _chemistry_type: &str,
        ) -> Result<CircularEconomyMetrics, RepositoryError> {
            unimplemented!()
        }
    }

    #[test]
    fn test_recycling_hash_integrity() {
        let mut cert = RecyclingCertification::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            "recycler-001".to_string(),
            "hydrometallurgical".to_string(),
            8.5,
            "ISO 14040".to_string(),
        );

        let original_hash = cert.compute_hash();
        cert.certification_hash = original_hash.clone();

        assert!(cert.verify_hash_integrity());

        // Tamper: change recovery rate
        cert.recovery_rates.lithium_percent = 50.0;
        assert!(!cert.verify_hash_integrity());
    }

    #[tokio::test]
    async fn test_recycling_service_valid_recovery() {
        let repo = std::sync::Arc::new(MockRecyclingRepo);
        let service = RecyclingServiceImpl::new(repo);

        let recovery = RecoveryRates {
            lithium_percent: 95.0,
            cobalt_percent: 99.0,
            nickel_percent: 94.0,
            other_percent: 85.0,
        };

        let result = service
            .record_recycling(
                "MY008A6FKKKLC1DH80001",
                "recycler-001",
                "hydrometallurgical",
                8.5,
                "ISO 14040",
                recovery,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recycling_service_invalid_recovery_high() {
        let repo = std::sync::Arc::new(MockRecyclingRepo);
        let service = RecyclingServiceImpl::new(repo);

        let recovery = RecoveryRates {
            lithium_percent: 150.0, // Invalid!
            cobalt_percent: 99.0,
            nickel_percent: 94.0,
            other_percent: 85.0,
        };

        let result = service
            .record_recycling(
                "MY008A6FKKKLC1DH80001",
                "recycler-001",
                "hydrometallurgical",
                8.5,
                "ISO 14040",
                recovery,
            )
            .await;

        assert!(result.is_err());
    }
}
