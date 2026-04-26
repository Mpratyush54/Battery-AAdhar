//! main.rs — Battery Pack Aadhaar gRPC server
//!
//! Boots all 4 services (Crypto, Battery, Auth, Lifecycle) on 0.0.0.0:50051.
//! Secrets priority:
//!   1. Environment variables (already injected, e.g. via `infisical run --`)
//!   2. Infisical SDK (fetched at startup using INFISICAL_CLIENT_ID/SECRET from .env)

use std::env;
use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::{info, warn};

use bpa_engine::api::{
    AuthServiceImpl, AuthServiceServer, BatteryServiceImpl, BatteryServiceServer,
    CryptoServiceImpl, CryptoServiceServer, LifecycleServiceImpl, LifecycleServiceServer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // ── Load local .env (holds Infisical credentials, cert paths, etc.) ───────
    dotenvy::dotenv().ok();

    // ── Resolve secrets: env first, then Infisical SDK ────────────────────────
    let mut connection_string: Option<String> = env::var("DATABASE_URL").ok();
    let mut master_key: Option<String> = env::var("ENCRYPTION_KEY").ok();
    let mut jwt_secret_opt: Option<String> = env::var("JWT_SECRET").ok();

    if connection_string.is_none() || master_key.is_none() {
        if let (Ok(client_id), Ok(client_secret)) = (
            env::var("INFISICAL_CLIENT_ID"),
            env::var("INFISICAL_CLIENT_SECRET"),
        ) {
            info!("🔐 Authenticating with Infisical...");

            let project_id =
                env::var("INFISICAL_PROJECT_ID").expect("INFISICAL_PROJECT_ID must be set");
            let environment =
                env::var("INFISICAL_ENV").unwrap_or_else(|_| "dev".to_string());
            let host = env::var("INFISICAL_BASE_URL")
                .unwrap_or_else(|_| "https://app.infisical.com".to_string());

            use infisical::{AuthMethod, Client};
            use infisical::secrets::GetSecretRequest;

            let mut client = Client::builder()
                .base_url(host)
                .build()
                .await
                .expect("Failed to build Infisical client");

            let auth_method = AuthMethod::new_universal_auth(&client_id, &client_secret);
            client
                .login(auth_method)
                .await
                .expect("❌ Infisical authentication failed");
            info!("✅ Infisical authentication successful");

            // DATABASE_URL
            if connection_string.is_none() {
                let req =
                    GetSecretRequest::builder("DATABASE_URL", &project_id, &environment).build();
                match client.secrets().get(req).await {
                    Ok(secret) => {
                        info!("✅ DATABASE_URL retrieved from Infisical");
                        connection_string = Some(secret.secret_value);
                    }
                    Err(e) => warn!("⚠️ Failed to fetch DATABASE_URL: {}", e),
                }
            }

            // ENCRYPTION_KEY
            if master_key.is_none() {
                let req =
                    GetSecretRequest::builder("ENCRYPTION_KEY", &project_id, &environment).build();
                match client.secrets().get(req).await {
                    Ok(secret) => {
                        info!("✅ ENCRYPTION_KEY retrieved from Infisical");
                        master_key = Some(secret.secret_value);
                    }
                    Err(e) => warn!("⚠️ Failed to fetch ENCRYPTION_KEY: {}", e),
                }
            }

            // JWT_SECRET
            if jwt_secret_opt.is_none() {
                let req =
                    GetSecretRequest::builder("JWT_SECRET", &project_id, &environment).build();
                match client.secrets().get(req).await {
                    Ok(secret) => {
                        info!("✅ JWT_SECRET retrieved from Infisical");
                        jwt_secret_opt = Some(secret.secret_value);
                    }
                    Err(e) => warn!("⚠️ Failed to fetch JWT_SECRET: {}", e),
                }
            }
        }
    }

    // ── Validate ──────────────────────────────────────────────────────────────
    let db_url = connection_string.expect(
        "DATABASE_URL not available — set via env or ensure INFISICAL_CLIENT_ID/SECRET are in .env",
    );
    let root_key_str = master_key.expect(
        "ENCRYPTION_KEY not available — set via env or ensure INFISICAL_CLIENT_ID/SECRET are in .env",
    );
    let jwt_secret = jwt_secret_opt.unwrap_or_else(|| "secret".to_string());

    // ── Decode ENCRYPTION_KEY to raw 32 bytes for KeyManager ─────────────────
    let mut root_key_bytes = [0u8; 32];
    if root_key_str.len() == 64 {
        for i in 0..32 {
            root_key_bytes[i] = u8::from_str_radix(&root_key_str[i * 2..i * 2 + 2], 16)
                .expect("ENCRYPTION_KEY must be valid hex");
        }
    } else if root_key_str.len() == 32 {
        root_key_bytes.copy_from_slice(root_key_str.as_bytes());
    } else {
        panic!(
            "ENCRYPTION_KEY must be 64 hex chars or 32 ASCII chars, got {} chars",
            root_key_str.len()
        );
    }

    // ── Connect to database ───────────────────────────────────────────────────
    use bpa_engine::services::encryption::EncryptionService;
    use sqlx::PgPool;

    info!("🗄️  Connecting to database...");
    let db_pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to DB");
    info!("✅ Database connected");

    // ── Initialise BPA engine ─────────────────────────────────────────────────
    let encryption =
        EncryptionService::new(&root_key_str).expect("Failed to create EncryptionService");

    let engine = std::sync::Arc::new(
        bpa_engine::BpaEngine::new(db_pool, encryption, jwt_secret, &root_key_bytes)
            .expect("Failed to initialize BPA Engine"),
    );

    engine.health_check().expect("Health check failed");
    info!("✓ BPA engine initialised and healthy");

    // ── Start gRPC server ─────────────────────────────────────────────────────
    let addr: SocketAddr = "0.0.0.0:50051".parse()?;
    info!("🚀 BPA gRPC server starting on {}", addr);

    let crypto_svc = CryptoServiceImpl::new(engine.clone());
    let battery_svc = BatteryServiceImpl::new(engine.clone());
    let auth_svc = AuthServiceImpl::new(engine.clone());
    let lifecycle_svc = LifecycleServiceImpl::new(engine.clone());

    Server::builder()
        .add_service(CryptoServiceServer::new(crypto_svc))
        .add_service(BatteryServiceServer::new(battery_svc))
        .add_service(AuthServiceServer::new(auth_svc))
        .add_service(LifecycleServiceServer::new(lifecycle_svc))
        .serve(addr)
        .await?;

    Ok(())
}
