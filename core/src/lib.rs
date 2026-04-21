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

#[derive(Clone)]
pub struct BpaEngine {
    pub db_pool: Pool<Postgres>,
    pub encryption: EncryptionService,
    pub registration: RegistrationService,
}

impl BpaEngine {
    pub fn new(db_pool: Pool<Postgres>, encryption: EncryptionService, _jwt_secret: String) -> Self {
        Self {
            registration: RegistrationService::new(db_pool.clone(), encryption.clone()),
            encryption,
            db_pool,
        }
    }
}
