#![allow(non_snake_case)]
pub mod api;
pub mod errors;
pub mod models;
pub mod repositories;
pub mod services;

pub mod common_v1 {
    tonic::include_proto!("bpa.common.v1");
}
pub mod crypto_v1 {
    tonic::include_proto!("bpa.crypto.v1");
}
pub mod battery_v1 {
    tonic::include_proto!("bpa.battery.v1");
}
pub mod auth_v1 {
    tonic::include_proto!("bpa.auth.v1");
}
pub mod lifecycle_v1 {
    tonic::include_proto!("bpa.lifecycle.v1");
}
pub mod circular_economy_v1 {
    tonic::include_proto!("bpa.circular_economy.v1");
}

use services::encryption::EncryptionService;
use services::key_manager::KeyManagerImpl;
use services::material::MaterialService;
use services::registration::RegistrationService;
use services::signing::SigningServiceImpl;
use services::zk_proofs::ZkProverImpl;
use services::compliance::ComplianceServiceImpl;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

#[derive(Clone)]
pub struct BpaEngine {
    pub db_pool: Pool<Postgres>,
    pub encryption: EncryptionService,
    pub registration: RegistrationService,
    pub key_manager: Arc<KeyManagerImpl>,
    pub signing_service: Arc<SigningServiceImpl>,
    pub zk_prover: Arc<ZkProverImpl>,
    pub material_service: MaterialService,
    pub lifecycle_service: Arc<crate::services::battery_lifecycle::BatteryLifecycleService>,
    pub reuse_service: Arc<dyn crate::services::reuse::ReuseService>,
    pub recycling_service: Arc<dyn crate::services::recycling::RecyclingService>,
    pub reuse_repo: Arc<dyn crate::repositories::reuse_repo::ReuseRepository>,
    pub recycling_repo: Arc<dyn crate::repositories::recycling_repo::RecyclingRepository>,
    pub compliance_service: Arc<ComplianceServiceImpl>,
}

impl BpaEngine {
    pub fn new(
        db_pool: Pool<Postgres>,
        encryption: EncryptionService,
        _jwt_secret: String,
        root_key_bytes: &[u8; 32],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let key_manager = Arc::new(KeyManagerImpl::new(root_key_bytes)?);
        let signing_service = Arc::new(SigningServiceImpl::new());
        let zk_prover = Arc::new(ZkProverImpl::new());
        let material_service = MaterialService::new(encryption.clone());

        let ownership_repo = Arc::new(
            crate::repositories::ownership_repo::OwnershipRepositoryImpl::new(db_pool.clone()),
        );
        let lifecycle_service = Arc::new(
            crate::services::battery_lifecycle::BatteryLifecycleService::new(ownership_repo),
        );

        let reuse_repo = Arc::new(
            crate::repositories::reuse_repo::ReuseRepositoryImpl::new(db_pool.clone()),
        );
        let recycling_repo = Arc::new(
            crate::repositories::recycling_repo::RecyclingRepositoryImpl::new(db_pool.clone()),
        );

        let reuse_service: Arc<dyn crate::services::reuse::ReuseService> = Arc::new(
            crate::services::reuse::ReuseServiceImpl::new(reuse_repo.clone()),
        );
        let recycling_service: Arc<dyn crate::services::recycling::RecyclingService> = Arc::new(
            crate::services::recycling::RecyclingServiceImpl::new(recycling_repo.clone()),
        );

        let health_repo = Arc::new(
            crate::repositories::health_repo::HealthRepositoryImpl::new(db_pool.clone()),
        );
        let compliance_repo = Arc::new(
            crate::repositories::compliance_repo::ComplianceRepositoryImpl::new(db_pool.clone()),
        );
        let material_repo = Arc::new(
            crate::repositories::material_repo::MaterialRepositoryImpl::new(db_pool.clone()),
        );
        let carbon_repo = Arc::new(
            crate::repositories::carbon_repo::CarbonRepositoryImpl::new(db_pool.clone()),
        );
        let battery_repo = Arc::new(
            crate::repositories::battery_repo::BatteryRepositoryImpl::new(db_pool.clone()),
        );

        let compliance_service = Arc::new(ComplianceServiceImpl::new(
            zk_prover.clone(),
            health_repo,
            compliance_repo,
            material_repo,
            carbon_repo,
            battery_repo,
        ));

        Ok(Self {
            registration: RegistrationService::new(db_pool.clone(), encryption.clone()),
            encryption,
            db_pool,
            key_manager,
            signing_service,
            zk_prover,
            material_service,
            lifecycle_service,
            reuse_service,
            recycling_service,
            reuse_repo,
            recycling_repo,
            compliance_service,
        })
    }

    /// Health check — verify all services are responsive
    pub fn health_check(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Quick smoke tests for each service
        let (_, _) = SigningServiceImpl::generate_keypair()?;
        let _ = self.zk_prover.prove_operational(85)?;

        Ok(())
    }
}
