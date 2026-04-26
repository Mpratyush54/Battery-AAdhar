#![allow(non_snake_case)]
pub mod errors;
pub mod models;
pub mod repositories;
pub mod services;
pub mod api;

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

use sqlx::{Pool, Postgres};
use services::encryption::EncryptionService;
use services::registration::RegistrationService;
use services::key_manager::KeyManagerImpl;
use services::signing::SigningServiceImpl;
use services::zk_proofs::ZkProverImpl;
use std::sync::Arc;

#[derive(Clone)]
pub struct BpaEngine {
    pub db_pool: Pool<Postgres>,
    pub encryption: EncryptionService,
    pub registration: RegistrationService,
    pub key_manager: Arc<KeyManagerImpl>,
    pub signing_service: Arc<SigningServiceImpl>,
    pub zk_prover: Arc<ZkProverImpl>,
}

impl BpaEngine {
    pub fn new(db_pool: Pool<Postgres>, encryption: EncryptionService, _jwt_secret: String, root_key_bytes: &[u8; 32]) -> Result<Self, Box<dyn std::error::Error>> {
        let key_manager = Arc::new(KeyManagerImpl::new(root_key_bytes)?);
        let signing_service = Arc::new(SigningServiceImpl::new());
        let zk_prover = Arc::new(ZkProverImpl::new());
        
        Ok(Self {
            registration: RegistrationService::new(db_pool.clone(), encryption.clone()),
            encryption,
            db_pool,
            key_manager,
            signing_service,
            zk_prover,
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

