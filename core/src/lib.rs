#![allow(non_snake_case)]
pub mod errors;
pub mod models;
pub mod repositories;
pub mod services;
pub mod api;

pub mod bpa {
    tonic::include_proto!("bpa");
}

use sqlx::{Pool, Postgres};
use services::encryption::EncryptionService;
use services::registration::RegistrationService;
use services::static_data::StaticDataService;
use services::dynamic_data::DynamicDataService;
use services::ownership::OwnershipService;
use services::reuse::ReuseService;
use services::recycling::RecyclingService;
use services::carbon_footprint::CarbonFootprintService;
use services::compliance::ComplianceService;
use services::access_control::AccessControlService;

#[derive(Clone)]
pub struct BpaEngine {
    pub db_pool: Pool<Postgres>,
    pub encryption: EncryptionService,
    pub auth_service: crate::services::auth::AuthService,
    pub registration: RegistrationService,
    pub static_data: StaticDataService,
    pub dynamic_data: DynamicDataService,
    pub ownership: OwnershipService,
    pub reuse: ReuseService,
    pub recycling: RecyclingService,
    pub carbon_footprint: CarbonFootprintService,
    pub compliance: ComplianceService,
    pub access_control: AccessControlService,
}

impl BpaEngine {
    pub fn new(db_pool: Pool<Postgres>, encryption: EncryptionService, jwt_secret: String) -> Self {
        Self {
            auth_service: crate::services::auth::AuthService::new(db_pool.clone(), jwt_secret),
            registration: RegistrationService::new(db_pool.clone(), encryption.clone()),
            static_data: StaticDataService::new(db_pool.clone(), encryption.clone()),
            dynamic_data: DynamicDataService::new(db_pool.clone(), encryption.clone()),
            ownership: OwnershipService::new(db_pool.clone(), encryption.clone()),
            reuse: ReuseService::new(db_pool.clone()),
            recycling: RecyclingService::new(db_pool.clone()),
            carbon_footprint: CarbonFootprintService::new(db_pool.clone()),
            compliance: ComplianceService::new(db_pool.clone()),
            access_control: AccessControlService::new(db_pool.clone()),
            encryption,
            db_pool,
        }
    }
}
