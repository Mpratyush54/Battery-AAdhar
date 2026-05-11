//! reuse.rs — Second-life battery certification (SoH 60–80%)
//!
//! Validates eligibility and creates immutable certification log entry.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReuseCertification {
    pub id: Uuid,
    pub bpan: String,
    pub soh_at_certification: f32,
    pub certified_by: String, // Reuse operator ID
    pub certified_at: DateTime<Utc>,
    pub intended_application: String, // "stationary_storage", "grid_backup", "renewable_integration"
    pub expected_second_life_years: u8,
    pub certification_hash: String, // SHA256 for integrity
}

impl ReuseCertification {
    pub fn new(
        bpan: String,
        soh: f32,
        certified_by: String,
        application: String,
        expected_years: u8,
    ) -> Self {
        ReuseCertification {
            id: Uuid::new_v4(),
            bpan,
            soh_at_certification: soh,
            certified_by,
            certified_at: Utc::now(),
            intended_application: application,
            expected_second_life_years: expected_years,
            certification_hash: String::new(),
        }
    }

    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.bpan.as_bytes());
        hasher.update(self.soh_at_certification.to_le_bytes());
        hasher.update(self.certified_by.as_bytes());
        hasher.update(self.certified_at.to_rfc3339().as_bytes());
        hasher.update(self.intended_application.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn verify_hash_integrity(&self) -> bool {
        self.certification_hash == self.compute_hash()
    }
}

#[derive(Debug)]
pub enum ReuseError {
    InvalidSoH(String),
    UnauthorizedRole(String),
    StorageError(String),
}

impl std::fmt::Display for ReuseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReuseError::InvalidSoH(msg) => write!(f, "invalid SoH: {}", msg),
            ReuseError::UnauthorizedRole(msg) => write!(f, "unauthorized: {}", msg),
            ReuseError::StorageError(msg) => write!(f, "storage error: {}", msg),
        }
    }
}

impl std::error::Error for ReuseError {}

#[async_trait]
pub trait ReuseService: Send + Sync {
    /// Certify battery for second-life (SoH must be 60–80%)
    async fn certify_second_life(
        &self,
        bpan: &str,
        current_soh: f32,
        certified_by: &str,
        application: &str,
        expected_years: u8,
    ) -> Result<String, ReuseError>; // Returns certification ID
}

pub struct ReuseServiceImpl {
    repo: std::sync::Arc<dyn crate::repositories::reuse_repo::ReuseRepository>,
}

impl ReuseServiceImpl {
    pub fn new(repo: std::sync::Arc<dyn crate::repositories::reuse_repo::ReuseRepository>) -> Self {
        ReuseServiceImpl { repo }
    }
}

#[async_trait]
impl ReuseService for ReuseServiceImpl {
    async fn certify_second_life(
        &self,
        bpan: &str,
        current_soh: f32,
        certified_by: &str,
        application: &str,
        expected_years: u8,
    ) -> Result<String, ReuseError> {
        // Validate SoH is in second-life range (60–80%)
        if current_soh < 60.0 || current_soh > 80.0 {
            return Err(ReuseError::InvalidSoH(
                format!("SoH must be 60–80%, got {}", current_soh),
            ));
        }

        // Create certification
        let mut cert = ReuseCertification::new(
            bpan.to_string(),
            current_soh,
            certified_by.to_string(),
            application.to_string(),
            expected_years,
        );
        cert.certification_hash = cert.compute_hash();

        // Store in DB via repository
        self.repo.insert_certification(
            &cert.bpan,
            cert.soh_at_certification,
            &cert.certified_by,
            &cert.intended_application,
            cert.expected_second_life_years as i32,
            &cert.certification_hash,
        ).await.map_err(|e| ReuseError::StorageError(e.to_string()))?;

        tracing::info!(
            "second-life certified: {} SoH={}% application={} expected_years={}",
            bpan,
            current_soh,
            application,
            expected_years
        );

        Ok(cert.id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::repositories::battery_repo::RepositoryError;

    struct MockReuseRepo;

    #[async_trait]
    impl crate::repositories::reuse_repo::ReuseRepository for MockReuseRepo {
        async fn insert_certification(
            &self,
            _bpan: &str,
            _soh: f32,
            _certified_by: &str,
            _application: &str,
            _expected_years: i32,
            _cert_hash: &str,
        ) -> Result<String, RepositoryError> {
            Ok(uuid::Uuid::new_v4().to_string())
        }

        async fn get_certifications(
            &self,
            _bpan: &str,
        ) -> Result<Vec<(String, f32, String, String)>, RepositoryError> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_reuse_certification_valid_soh() {
        let cert = ReuseCertification::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            75.0, // Valid: 60–80%
            "reuse-op-001".to_string(),
            "stationary_storage".to_string(),
            5,
        );

        assert!(cert.soh_at_certification >= 60.0 && cert.soh_at_certification <= 80.0);
    }

    #[test]
    fn test_reuse_hash_integrity() {
        let mut cert = ReuseCertification::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            75.0,
            "reuse-op-001".to_string(),
            "stationary_storage".to_string(),
            5,
        );
        cert.certification_hash = cert.compute_hash();

        assert!(cert.verify_hash_integrity());
    }

    #[tokio::test]
    async fn test_reuse_service_invalid_soh_high() {
        let repo = std::sync::Arc::new(MockReuseRepo);
        let service = ReuseServiceImpl::new(repo);

        let result = service
            .certify_second_life(
                "MY008A6FKKKLC1DH80001",
                85.0, // Too high!
                "reuse-op-001",
                "stationary_storage",
                5,
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(ReuseError::InvalidSoH(_)) => (),
            _ => panic!("expected InvalidSoH error"),
        }
    }

    #[tokio::test]
    async fn test_reuse_service_invalid_soh_low() {
        let repo = std::sync::Arc::new(MockReuseRepo);
        let service = ReuseServiceImpl::new(repo);

        let result = service
            .certify_second_life(
                "MY008A6FKKKLC1DH80001",
                50.0, // Too low!
                "reuse-op-001",
                "stationary_storage",
                5,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reuse_service_valid_certification() {
        let repo = std::sync::Arc::new(MockReuseRepo);
        let service = ReuseServiceImpl::new(repo);

        let result = service
            .certify_second_life(
                "MY008A6FKKKLC1DH80001",
                72.5, // Valid!
                "reuse-op-001",
                "grid_backup",
                5,
            )
            .await;

        assert!(result.is_ok());
    }
}
