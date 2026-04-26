//! main.rs — Battery Pack Aadhaar gRPC server
//!
//! Boots all 4 services (Crypto, Battery, Auth, Lifecycle) on [::1]:50051

use std::net::SocketAddr;
use tonic::transport::Server;

use std::env;

// Import all service implementations
use bpa_engine::api::{
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
        panic!("ENCRYPTION_KEY environment variable is not set");
    }

    // Decode to raw 32 bytes for BpaEngine (KeyManager uses raw bytes)
    let mut root_key_bytes = [0u8; 32];
    if root_key_str.len() == 64 {
        for i in 0..32 {
            root_key_bytes[i] = u8::from_str_radix(&root_key_str[i * 2..i * 2 + 2], 16)
                .expect("ENCRYPTION_KEY must be valid hex");
        }
    } else if root_key_str.len() == 32 {
        root_key_bytes.copy_from_slice(root_key_str.as_bytes());
    } else {
        panic!("ENCRYPTION_KEY must be 64 hex chars or 32 ASCII chars, got {} chars", root_key_str.len());
    }

    use sqlx::PgPool;
    use bpa_engine::services::encryption::EncryptionService;

    // Connect to DB
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
    let db_pool = PgPool::connect(&db_url).await.expect("Failed to connect to DB");
    
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    // EncryptionService::new accepts both 32-char ASCII and 64-char hex
    let encryption = EncryptionService::new(&root_key_str).expect("Failed to create EncryptionService");

    let engine = std::sync::Arc::new(
        bpa_engine::BpaEngine::new(db_pool, encryption, jwt_secret, &root_key_bytes)
            .expect("Failed to initialize BPA Engine")
    );

    engine.health_check().expect("Health check failed");
    tracing::info!("✓ BPA engine initialized and healthy");

    let addr: SocketAddr = "0.0.0.0:50051".parse()?;
    tracing::info!("BPA gRPC server starting on {}", addr);

    // Create service instances
    let crypto_svc = CryptoServiceImpl::new(engine.clone());
    let battery_svc = BatteryServiceImpl::new(engine.clone());
    let auth_svc = AuthServiceImpl::new(engine.clone());
    let lifecycle_svc = LifecycleServiceImpl::new(engine.clone());

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
