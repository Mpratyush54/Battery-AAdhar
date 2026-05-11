use bpa_engine::services::reuse::{ReuseService, ReuseServiceImpl};
use bpa_engine::services::recycling::{RecyclingService, RecyclingServiceImpl, RecoveryRates};
use bpa_engine::repositories::reuse_repo::ReuseRepository;
use bpa_engine::repositories::recycling_repo::{RecyclingRepository, CircularEconomyMetrics};
use async_trait::async_trait;
use bpa_engine::repositories::battery_repo::RepositoryError;

struct MockReuseRepo;

#[async_trait]
impl ReuseRepository for MockReuseRepo {
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

struct MockRecyclingRepo;

#[async_trait]
impl RecyclingRepository for MockRecyclingRepo {
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
        Ok(CircularEconomyMetrics {
            battery_count: 10,
            avg_li_recovery: 95.0,
            avg_co_recovery: 98.0,
            avg_ni_recovery: 93.0,
            total_weight_processed_kg: 150.0,
        })
    }

    async fn get_metrics_by_chemistry(
        &self,
        _chemistry_type: &str,
    ) -> Result<CircularEconomyMetrics, RepositoryError> {
        Ok(CircularEconomyMetrics {
            battery_count: 5,
            avg_li_recovery: 96.0,
            avg_co_recovery: 99.0,
            avg_ni_recovery: 94.0,
            total_weight_processed_kg: 75.0,
        })
    }
}

#[tokio::test]
async fn test_reuse_certification_flow() {
    let repo = std::sync::Arc::new(MockReuseRepo);
    let service = ReuseServiceImpl::new(repo);

    // 1. Success: SoH 70% (within 60-80 range)
    let result = service.certify_second_life(
        "MY008A6FKKKLC1DH80001",
        70.0,
        "operator-001",
        "stationary_storage",
        5
    ).await;
    assert!(result.is_ok(), "SoH 70% should be valid for second life");

    // 2. Failure: SoH 85% (Too high for second life)
    let result = service.certify_second_life(
        "MY008A6FKKKLC1DH80001",
        85.0,
        "operator-001",
        "stationary_storage",
        5
    ).await;
    assert!(result.is_err(), "SoH 85% should fail (too high)");

    // 3. Failure: SoH 50% (End of life, not second life)
    let result = service.certify_second_life(
        "MY008A6FKKKLC1DH80001",
        50.0,
        "operator-001",
        "stationary_storage",
        5
    ).await;
    assert!(result.is_err(), "SoH 50% should fail (too low)");
}

#[tokio::test]
async fn test_recycling_record_flow() {
    let repo = std::sync::Arc::new(MockRecyclingRepo);
    let service = RecyclingServiceImpl::new(repo);

    let rates = RecoveryRates {
        lithium_percent: 95.5,
        cobalt_percent: 99.1,
        nickel_percent: 94.2,
        other_percent: 88.0,
    };

    // 1. Success
    let result = service.record_recycling(
        "MY008A6FKKKLC1DH80001",
        "recycler-001",
        "hydrometallurgical",
        12.5,
        "ISO 14040",
        rates
    ).await;
    assert!(result.is_ok(), "Valid recycling record should be accepted");

    // 2. Failure: Invalid recovery rate (> 100)
    let bad_rates = RecoveryRates {
        lithium_percent: 105.0,
        ..RecoveryRates::default()
    };
    let result = service.record_recycling(
        "MY008A6FKKKLC1DH80001",
        "recycler-001",
        "hydrometallurgical",
        12.5,
        "ISO 14040",
        bad_rates
    ).await;
    assert!(result.is_err(), "Recovery rate > 100% should fail");
}
