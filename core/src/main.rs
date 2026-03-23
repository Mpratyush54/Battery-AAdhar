use dotenvy::dotenv;
use std::env;
use tonic::transport::Server;
use tracing::{info, warn};
use infisical::{AuthMethod, Client};
use infisical::secrets::GetSecretRequest;

use sqlx::postgres::PgPoolOptions;

use bpa_engine::repositories::db_setup::sync_database_schema;
use bpa_engine::services::encryption::EncryptionService;
use bpa_engine::BpaEngine;
use bpa_engine::bpa::bpa_service_server::BpaServiceServer;







#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    info!("🔋 Battery Pack Aadhaar — Core Engine starting...");

    // --- Load secrets from env or .env first ---
    let mut connection_string = env::var("DATABASE_URL").ok();
    let mut master_key = env::var("ENCRYPTION_KEY").ok();

    // --- Try Infisical if secrets not already present ---
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
            println!("Client ID = {:?}", env::var("INFISICAL_CLIENT_ID"));
            println!("Base URL = {:?}", env::var("INFISICAL_BASE_URL"));
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

            // Fetch DATABASE_URL
            if connection_string.is_none() {
                let req = GetSecretRequest::builder("DATABASE_URL", &project_id, &environment)
                    .build();

                match client.secrets().get(req).await {
                    Ok(secret) => {
                        info!("✅ DATABASE_URL retrieved from Infisical");
                        connection_string = Some(secret.secret_value);
                    }
                    Err(e) => warn!("⚠️ Failed to fetch DATABASE_URL: {}", e),
                }
            }

            // Fetch ENCRYPTION_KEY
            if master_key.is_none() {
                let req = GetSecretRequest::builder("ENCRYPTION_KEY", &project_id, &environment)
                    .build();

                match client.secrets().get(req).await {
                    Ok(secret) => {
                        info!("✅ ENCRYPTION_KEY retrieved from Infisical");
                        master_key = Some(secret.secret_value);
                    }
                    Err(e) => warn!("⚠️ Failed to fetch ENCRYPTION_KEY: {}", e),
                }
            }
        }
    }

    // --- Ensure required secrets exist ---
    let connection_string =
        connection_string.expect("DATABASE_URL must be set (via .env, env vars, or Infisical)");

    let master_key =
        master_key.expect("ENCRYPTION_KEY must be set (via .env, env vars, or Infisical)");

    // --- Initialize encryption ---
    let encryption = EncryptionService::new(&master_key)
        .expect("Failed to initialize encryption service");

    // --- Connect to database ---
    info!("📡 Connecting to database...");

    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&connection_string)
        .await
        .expect("Failed to initialize database pool");

    info!("✅ Database connected");

    // --- Auto-sync schema ---
    sync_database_schema(&db_pool)
        .await
        .expect("Failed to sync database schema");

    info!("✅ Database schema ready");

    // --- Initialize BPA Engine ---
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| master_key.clone());
    let engine = BpaEngine::new(db_pool, encryption, jwt_secret);

    let listen_address = "127.0.0.1:50051".parse()?;

    info!("🚀 BPA Core Engine ready on {}", listen_address);
    info!("   ├── Registration Service ✓");
    info!("   ├── Static Data Service ✓");
    info!("   ├── Dynamic Data Service ✓");
    info!("   ├── Ownership Service ✓");
    info!("   ├── Reuse Service ✓");
    info!("   ├── Recycling Service ✓");
    info!("   ├── Carbon Footprint Service ✓");
    info!("   ├── Compliance Service ✓");
    info!("   ├── Access Control Service ✓");
    info!("   ├── BPAN Generator ✓");
    info!("   ├── QR Service ✓");
    info!("   ├── Hash Chain Service ✓");
    info!("   ├── Encryption Service ✓");
    info!("   └── Validation Service ✓");

    Server::builder()
        .add_service(BpaServiceServer::new(engine.clone()))
        .serve(listen_address)
        .await?;

    Ok(())
}
