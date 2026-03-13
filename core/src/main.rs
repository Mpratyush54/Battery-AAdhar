#![allow(non_snake_case)]
pub mod errors;
pub mod models;
pub mod repositories;
pub mod services;

use dotenvy::dotenv;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::env;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, error, warn};
use infisical::{AuthMethod, Client};
use infisical::secrets::GetSecretRequest;

use crate::services::encryption::EncryptionService;
use crate::services::registration::RegistrationService;
use crate::services::static_data::StaticDataService;
use crate::services::dynamic_data::DynamicDataService;
use crate::services::ownership::OwnershipService;
use crate::services::reuse::ReuseService;
use crate::services::recycling::RecyclingService;
use crate::services::carbon_footprint::CarbonFootprintService;
use crate::services::compliance::ComplianceService;
use crate::services::access_control::AccessControlService;

// Import the old ride proto for backward compatibility.
// In future, this will be replaced with a proper BPA proto.
use rides::ride_service_server::{RideService, RideServiceServer};
use rides::{CreateRideRequest, GetRidesRequest, GetRidesResponse, RideResponse};

pub mod rides {
    tonic::include_proto!("rides");
}

/// The BPA Core Engine — holds the DB pool, encryption service, and all domain services.
pub struct BpaEngine {
    db_pool: Pool<Postgres>,
    encryption: EncryptionService,
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
    pub fn new(db_pool: Pool<Postgres>, encryption: EncryptionService) -> Self {
        Self {
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

// Backward-compatible RideService implementation using the new encryption service
#[tonic::async_trait]
impl RideService for BpaEngine {
    async fn create_ride(
        &self,
        request: Request<CreateRideRequest>,
    ) -> Result<Response<RideResponse>, Status> {
        let req_payload = request.into_inner();
        let encrypted_string = self.encryption.encrypt(&req_payload.ride_details)
            .map_err(|e| Status::internal(e.to_string()))?;

        let record = sqlx::query(
            r#"
            INSERT INTO rides (zk_proof, encrypted_details)
            VALUES ($1, $2)
            RETURNING id, zk_proof, encrypted_details
            "#,
        )
        .bind(&req_payload.zk_proof)
        .bind(&encrypted_string)
        .fetch_one(&self.db_pool)
        .await;

        match record {
            Ok(row) => {
                let json_response = RideResponse {
                    id: row.get("id"),
                    zk_proof: row.get("zk_proof"),
                    ride_details: req_payload.ride_details,
                };
                Ok(Response::new(json_response))
            }
            Err(err) => {
                error!("Database Error: {}", err);
                Err(Status::internal("Failed to insert ride"))
            }
        }
    }

    async fn get_rides(
        &self,
        _request: Request<GetRidesRequest>,
    ) -> Result<Response<GetRidesResponse>, Status> {
        let records = sqlx::query(
            "SELECT id, zk_proof, encrypted_details FROM rides"
        )
        .fetch_all(&self.db_pool)
        .await;

        match records {
            Ok(rows) => {
                let mut response_array = Vec::new();
                for row in rows {
                    let encrypted_details: String = row.get("encrypted_details");
                    let decrypted_string = self
                        .encryption
                        .decrypt(&encrypted_details)
                        .unwrap_or_else(|_| "[Decryption Error]".to_string());

                    response_array.push(RideResponse {
                        id: row.get("id"),
                        zk_proof: row.get("zk_proof"),
                        ride_details: decrypted_string,
                    });
                }
                Ok(Response::new(GetRidesResponse { rides: response_array }))
            }
            Err(_) => Err(Status::internal("Failed to query database")),
        }
    }
}

/// Create all database tables for the BPA system.
/// This runs on startup and uses IF NOT EXISTS so it's safe to call repeatedly.
async fn sync_database_schema(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    info!("Synchronizing database schema...");

    // Legacy rides table (backward compat)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS rides (
            id SERIAL PRIMARY KEY,
            zk_proof TEXT NOT NULL,
            encrypted_details TEXT NOT NULL
        );
    "#).execute(pool).await?;

    // --- Core BPA tables ---

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS manufacturers (
            id UUID PRIMARY KEY,
            manufacturer_code VARCHAR(10) NOT NULL UNIQUE,
            name VARCHAR(255) NOT NULL,
            country_code VARCHAR(2) NOT NULL,
            encrypted_profile TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS batteries (
            bpan VARCHAR(21) PRIMARY KEY,
            manufacturer_id UUID NOT NULL,
            production_year INT NOT NULL,
            battery_category VARCHAR(50) NOT NULL,
            compliance_class VARCHAR(50) NOT NULL,
            static_hash VARCHAR(64) NOT NULL,
            carbon_hash VARCHAR(64) NOT NULL DEFAULT 'PENDING',
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS battery_identifiers (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            encrypted_serial_number TEXT NOT NULL,
            encrypted_batch_number TEXT NOT NULL,
            encrypted_factory_code TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS battery_descriptor (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            chemistry_type VARCHAR(50) NOT NULL,
            nominal_voltage DOUBLE PRECISION NOT NULL,
            rated_capacity_kwh DOUBLE PRECISION NOT NULL,
            energy_density DOUBLE PRECISION NOT NULL,
            weight_kg DOUBLE PRECISION NOT NULL,
            form_factor VARCHAR(50) NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS battery_material_composition (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            cathode_material VARCHAR(100) NOT NULL,
            anode_material VARCHAR(100) NOT NULL,
            electrolyte_type VARCHAR(100) NOT NULL,
            separator_material VARCHAR(100) NOT NULL,
            lithium_content_g DOUBLE PRECISION NOT NULL,
            cobalt_content_g DOUBLE PRECISION NOT NULL,
            nickel_content_g DOUBLE PRECISION NOT NULL,
            recyclable_percentage DOUBLE PRECISION NOT NULL,
            encrypted_details TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS carbon_footprint (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            raw_material_emission DOUBLE PRECISION NOT NULL,
            manufacturing_emission DOUBLE PRECISION NOT NULL,
            transport_emission DOUBLE PRECISION NOT NULL,
            usage_emission DOUBLE PRECISION NOT NULL,
            recycling_emission DOUBLE PRECISION NOT NULL,
            total_emission DOUBLE PRECISION NOT NULL,
            verified BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS battery_health (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            state_of_health DOUBLE PRECISION NOT NULL,
            total_cycles INT NOT NULL,
            degradation_class VARCHAR(5) NOT NULL,
            end_of_life BOOLEAN NOT NULL DEFAULT FALSE,
            updated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS ownership_history (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            encrypted_owner_identity TEXT NOT NULL,
            start_time TIMESTAMP NOT NULL,
            end_time TIMESTAMP NOT NULL DEFAULT '9999-12-31 23:59:59'
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS reuse_history (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            reuse_application VARCHAR(255) NOT NULL,
            certified_by VARCHAR(255) NOT NULL,
            certified_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS recycling_records (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            recycler_name VARCHAR(255) NOT NULL,
            recovered_material_percentage DOUBLE PRECISION NOT NULL,
            certificate_hash VARCHAR(64) NOT NULL,
            recycled_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS telemetry (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            encrypted_payload TEXT NOT NULL,
            recorded_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS qr_records (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            qr_payload_hash VARCHAR(64) NOT NULL,
            generated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    // --- Stakeholder & Access Control ---

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS stakeholders (
            id UUID PRIMARY KEY,
            role VARCHAR(50) NOT NULL,
            encrypted_profile TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS data_access_control (
            id UUID PRIMARY KEY,
            stakeholder_id UUID NOT NULL,
            resource_type VARCHAR(100) NOT NULL,
            access_level VARCHAR(50) NOT NULL,
            UNIQUE(stakeholder_id, resource_type)
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS regulator_access_log (
            id UUID PRIMARY KEY,
            stakeholder_id UUID NOT NULL,
            bpan VARCHAR(21) NOT NULL,
            reason TEXT NOT NULL,
            accessed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    // --- Audit & Compliance Logs ---

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS audit_logs (
            id UUID PRIMARY KEY,
            actor_id UUID NOT NULL,
            action VARCHAR(100) NOT NULL,
            resource VARCHAR(255) NOT NULL,
            previous_hash VARCHAR(64) NOT NULL,
            entry_hash VARCHAR(64) NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS battery_registration_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            manufacturer_id UUID NOT NULL,
            registration_status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
            submitted_at TIMESTAMP NOT NULL DEFAULT NOW(),
            approved_at TIMESTAMP,
            approved_by UUID
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS static_data_submission_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            submitted_by UUID NOT NULL,
            data_section VARCHAR(100) NOT NULL,
            data_hash VARCHAR(64) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            submitted_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS validation_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            validation_type VARCHAR(100) NOT NULL,
            validation_result VARCHAR(50) NOT NULL,
            remarks TEXT NOT NULL DEFAULT '',
            validated_by UUID NOT NULL,
            validated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS static_data_update_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            updated_by UUID NOT NULL,
            field_name VARCHAR(100) NOT NULL,
            previous_hash VARCHAR(64) NOT NULL,
            new_hash VARCHAR(64) NOT NULL,
            updated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS dynamic_data_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            upload_type VARCHAR(50) NOT NULL,
            record_hash VARCHAR(64) NOT NULL,
            uploaded_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS ownership_transfer_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            from_owner_hash VARCHAR(64) NOT NULL,
            to_owner_hash VARCHAR(64) NOT NULL,
            transfer_reason VARCHAR(255) NOT NULL,
            transferred_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS reuse_certification_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            application_type VARCHAR(255) NOT NULL,
            certifier_hash VARCHAR(64) NOT NULL,
            certified_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS recycling_certification_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            recycler_hash VARCHAR(64) NOT NULL,
            material_recovery_hash VARCHAR(64) NOT NULL,
            certified_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS qr_generation_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            payload_hash VARCHAR(64) NOT NULL,
            generated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS data_access_execution_log (
            id UUID PRIMARY KEY,
            stakeholder_id UUID NOT NULL,
            bpan VARCHAR(21) NOT NULL,
            resource_type VARCHAR(100) NOT NULL,
            access_type VARCHAR(20) NOT NULL,
            granted BOOLEAN NOT NULL,
            accessed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS compliance_violation_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            violation_type VARCHAR(50) NOT NULL,
            severity VARCHAR(20) NOT NULL,
            detected_at TIMESTAMP NOT NULL DEFAULT NOW(),
            resolved BOOLEAN NOT NULL DEFAULT FALSE
        );
    "#).execute(pool).await?;

    // --- Cryptographic Infrastructure ---

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS certificates (
            id UUID PRIMARY KEY,
            public_key TEXT NOT NULL,
            issued_by_hash VARCHAR(64) NOT NULL,
            issued_at TIMESTAMP NOT NULL DEFAULT NOW(),
            expires_at TIMESTAMP NOT NULL,
            revoked BOOLEAN NOT NULL DEFAULT FALSE
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS certificate_revocation_list (
            id UUID PRIMARY KEY,
            certificate_id UUID NOT NULL,
            revoked_by_hash VARCHAR(64) NOT NULL,
            reason VARCHAR(255) NOT NULL,
            revoked_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS root_keys (
            id UUID PRIMARY KEY,
            key_identifier VARCHAR(100) NOT NULL UNIQUE,
            hardware_backed BOOLEAN NOT NULL DEFAULT FALSE,
            status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            retired_at TIMESTAMP NOT NULL DEFAULT '9999-12-31 23:59:59'
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS kek_keys (
            id UUID PRIMARY KEY,
            encrypted_kek BYTEA NOT NULL,
            version INT NOT NULL,
            root_key_id UUID NOT NULL,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            retired_at TIMESTAMP NOT NULL DEFAULT '9999-12-31 23:59:59'
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS battery_keys (
            bpan VARCHAR(21) PRIMARY KEY REFERENCES batteries(bpan),
            encrypted_dek BYTEA NOT NULL,
            kek_version INT NOT NULL,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            key_status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            rotated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS static_signatures (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            data_section VARCHAR(100) NOT NULL,
            data_hash VARCHAR(64) NOT NULL,
            signature BYTEA NOT NULL,
            certificate_id UUID NOT NULL,
            signed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS key_rotation_log (
            id UUID PRIMARY KEY,
            key_type VARCHAR(50) NOT NULL,
            previous_version INT NOT NULL,
            new_version INT NOT NULL,
            initiated_by UUID NOT NULL,
            approved_by UUID,
            approval_timestamp TIMESTAMP,
            rotated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS key_destruction_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            dek_version INT NOT NULL,
            destroyed_by UUID NOT NULL,
            destruction_method VARCHAR(50) NOT NULL,
            destroyed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    // --- Infrastructure & Observability ---

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS api_requests (
            id UUID PRIMARY KEY,
            parent_request_id UUID,
            request_hash VARCHAR(64) NOT NULL,
            endpoint_hash VARCHAR(64) NOT NULL,
            subject_hash VARCHAR(64) NOT NULL,
            status_hash VARCHAR(64) NOT NULL,
            latency_ms INT NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS idempotency_keys (
            id UUID PRIMARY KEY,
            request_hash VARCHAR(64) NOT NULL UNIQUE,
            response_hash VARCHAR(64) NOT NULL,
            expires_at TIMESTAMP NOT NULL
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS rate_limits (
            id UUID PRIMARY KEY,
            subject_hash VARCHAR(64) NOT NULL,
            window_start TIMESTAMP NOT NULL,
            request_count INT NOT NULL DEFAULT 0
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS scheduled_jobs (
            id UUID PRIMARY KEY,
            job_name_hash VARCHAR(64) NOT NULL UNIQUE,
            cron_expression VARCHAR(100) NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT TRUE,
            last_run TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS job_execution_log (
            id UUID PRIMARY KEY,
            job_id UUID NOT NULL,
            status VARCHAR(20) NOT NULL,
            duration_ms INT NOT NULL DEFAULT 0,
            executed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS notifications (
            id UUID PRIMARY KEY,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            recipient_hash VARCHAR(64) NOT NULL,
            encrypted_message TEXT NOT NULL,
            status_hash VARCHAR(64) NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS message_queue (
            id UUID PRIMARY KEY,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            topic_hash VARCHAR(64) NOT NULL,
            encrypted_payload TEXT NOT NULL,
            status_hash VARCHAR(64) NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS dead_letter_queue (
            id UUID PRIMARY KEY,
            original_message_id UUID NOT NULL,
            failure_reason_hash VARCHAR(64) NOT NULL,
            retry_count INT NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS system_metrics (
            id UUID PRIMARY KEY,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            metric_name_hash VARCHAR(64) NOT NULL,
            metric_value_cipher TEXT NOT NULL,
            recorded_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS alerts (
            id UUID PRIMARY KEY,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            severity_hash VARCHAR(64) NOT NULL,
            message_cipher TEXT NOT NULL,
            triggered_at TIMESTAMP NOT NULL DEFAULT NOW(),
            resolved BOOLEAN NOT NULL DEFAULT FALSE
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS regions (
            id UUID PRIMARY KEY,
            region_hash VARCHAR(64) NOT NULL,
            data_center_hash VARCHAR(64) NOT NULL
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS system_integrity_log (
            id UUID PRIMARY KEY,
            check_type VARCHAR(100) NOT NULL,
            status VARCHAR(20) NOT NULL,
            checked_at TIMESTAMP NOT NULL DEFAULT NOW()
        );
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS data_classification (
            id UUID PRIMARY KEY,
            table_name VARCHAR(100) NOT NULL,
            field_name VARCHAR(100) NOT NULL,
            classification VARCHAR(50) NOT NULL,
            UNIQUE(table_name, field_name)
        );
    "#).execute(pool).await?;

    // --- Performance indexes ---

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_battery_identifiers_bpan ON battery_identifiers(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_battery_descriptor_bpan ON battery_descriptor(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_battery_health_bpan ON battery_health(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_ownership_history_bpan ON ownership_history(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_telemetry_bpan ON telemetry(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_logs_actor ON audit_logs(actor_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_dynamic_data_log_bpan ON dynamic_data_log(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_carbon_footprint_bpan ON carbon_footprint(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_compliance_violations_bpan ON compliance_violation_log(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recycling_records_bpan ON recycling_records(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_reuse_history_bpan ON reuse_history(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_qr_records_bpan ON qr_records(bpan)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_battery_registration_bpan ON battery_registration_log(bpan)").execute(pool).await?;

    info!("✅ Database schema synchronized — {} tables created/verified", 42);
    Ok(())
}

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
    let engine = BpaEngine::new(db_pool, encryption);

    let listen_address = "[::1]:50051".parse()?;

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

    // --- Start gRPC server ---
    Server::builder()
        .add_service(RideServiceServer::new(engine))
        .serve(listen_address)
        .await?;

    Ok(())
}
