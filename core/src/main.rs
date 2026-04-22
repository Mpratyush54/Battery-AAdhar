//! main.rs — Battery Pack Aadhaar gRPC server
//!
//! Boots all 4 services (Crypto, Battery, Auth, Lifecycle) on [::1]:50051

use std::net::SocketAddr;
use tonic::transport::Server;

mod api;
mod models;
mod repositories;

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
