//! main.rs — Battery Pack Aadhaar gRPC server
//!
//! Boots all 4 services (Crypto, Battery, Auth, Lifecycle) on [::1]:50051

use std::net::SocketAddr;
use tonic::transport::Server;

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

    // Load or generate root key
    let root_key_str = env::var("ENCRYPTION_KEY")
        .unwrap_or_else(|_| "0000000000000000000000000000000000000000000000000000000000000000".to_string()); // Default for testing if missing

    if root_key_str.len() != 64 {
        panic!("ENCRYPTION_KEY must be 64 hex chars (32 bytes)");
    }

    let mut root_key_bytes = [0u8; 32];
    for i in 0..32 {
        root_key_bytes[i] = u8::from_str_radix(&root_key_str[i * 2..i * 2 + 2], 16)
            .expect("ENCRYPTION_KEY must be valid hex");
    }

    let key_manager = KeyManagerImpl::new(&root_key_bytes)
        .expect("Failed to initialize key manager");

    tracing::info!("✓ Key manager initialized");

    let addr: SocketAddr = "[::1]:50051".parse()?;
    tracing::info!("🚀 BPA gRPC server starting on {}", addr);

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
