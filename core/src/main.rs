//! main.rs — Battery Pack Aadhaar gRPC server
//!
//! Boots all 4 services (Crypto, Battery, Auth, Lifecycle) on [::1]:50051

use std::net::SocketAddr;
use tonic::transport::Server;

mod errors;
mod api;
mod models;
mod repositories;
mod services;

use services::key_manager::KeyManagerImpl;
use std::env;

// Import all service implementations
use api::{
    CryptoServiceImpl,
    BatteryServiceImpl,
    AuthServiceImpl,
    LifecycleServiceImpl,
    CryptoServiceServer,
    BatteryServiceServer,
    AuthServiceServer,
    LifecycleServiceServer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load .env file if it exists
    dotenvy::dotenv().ok();

    // Load or generate root key
    let root_key_str = env::var("ENCRYPTION_KEY").unwrap_or_default();
    
    if root_key_str.is_empty() {
        panic!("Failed to initialize key manager: RootKeyUnavailable (ENCRYPTION_KEY environment variable is not set)");
    }

    let mut root_key_bytes = [0u8; 32];
    if root_key_str.len() == 64 {
        for i in 0..32 {
            root_key_bytes[i] = u8::from_str_radix(&root_key_str[i * 2..i * 2 + 2], 16)
                .expect("ENCRYPTION_KEY must be valid hex");
        }
    } else if root_key_str.len() == 32 {
        root_key_bytes.copy_from_slice(root_key_str.as_bytes());
    } else {
        panic!("ENCRYPTION_KEY must be 64 hex chars or 32 ascii chars. Got length: {}", root_key_str.len());
    }

    let key_manager = KeyManagerImpl::new(&root_key_bytes)
        .expect("Failed to initialize key manager");

    tracing::info!("✓ Key manager initialized");

    let addr: SocketAddr = "0.0.0.0:50051".parse()?;
    tracing::info!("BPA gRPC server starting on {}", addr);

    // Create service instances
    let crypto_svc = CryptoServiceImpl;
    let battery_svc = BatteryServiceImpl;
    let auth_svc = AuthServiceImpl;
    let lifecycle_svc = LifecycleServiceImpl;

    // Build and start the server
    Server::builder()
        .add_service(CryptoServiceServer::new(crypto_svc))
        .add_service(BatteryServiceServer::new(battery_svc))
        .add_service(AuthServiceServer::new(auth_svc))
        .add_service(LifecycleServiceServer::new(lifecycle_svc))
        .serve(addr)
        .await?;

    Ok(())
}
