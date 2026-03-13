use chrono::Utc;
use sqlx::{Pool, Postgres};
use tracing::{info, warn, error, instrument};
use uuid::Uuid;

use crate::errors::{BpaError, BpaResult};
use crate::services::encryption::EncryptionService;
use crate::services::hash_chain::HashChainService;
use crate::services::battery_lifecycle::{BatteryLifecycleService, SohEvaluation};
use crate::services::validation::ValidationService;

/// Manages dynamic/lifecycle data for batteries (Layer 3 — server-based records).
/// This is the Phase 2 data layer that tracks real-time battery health,
/// telemetry events, thermal incidents, and state-of-health updates.
///
/// Per BPA guidelines, dynamic data is:
/// - Updated throughout the battery's operational life
/// - Accessible only to authorized stakeholders
/// - Immutably logged with hash chains for tamper evidence
#[derive(Clone)]
pub struct DynamicDataService {
    pool: Pool<Postgres>,
    encryption: EncryptionService,
}

impl DynamicDataService {
    pub fn new(pool: Pool<Postgres>, encryption: EncryptionService) -> Self {
        Self { pool, encryption }
    }

    /// Update the State of Health (SoH) for a battery.
    /// Automatically evaluates whether the battery should be flagged for reuse or EOL.
    #[instrument(name = "update_soh", skip(self))]
    pub async fn update_soh(
        &self,
        bpan: &str,
        state_of_health: f64,
        total_cycles: i32,
        degradation_class: &str,
        actor_id: Uuid,
    ) -> BpaResult<SohEvaluation> {
        // Validate inputs
        ValidationService::validate_soh(state_of_health)?;
        ValidationService::validate_cycle_count(total_cycles)?;
        ValidationService::validate_degradation_class(degradation_class)?;

        let now = Utc::now().naive_utc();
        let evaluation = BatteryLifecycleService::evaluate_soh(state_of_health)?;
        let is_eol = matches!(evaluation, SohEvaluation::EndOfLife);

        let mut tx = self.pool.begin().await?;

        // Update battery_health
        let update_query = "UPDATE battery_health SET state_of_health = $1, total_cycles = $2, degradation_class = $3, end_of_life = $4, updated_at = $5 WHERE bpan = $6";
        let result = sqlx::query(update_query)
            .bind(state_of_health)
            .bind(total_cycles)
            .bind(degradation_class)
            .bind(is_eol)
            .bind(&now)
            .bind(bpan)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(BpaError::NotFound(format!(
                "No battery_health record found for BPAN: {}",
                bpan
            )));
        }

        // Log as dynamic data event
        let log_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let event_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "UPDATE_SOH",
            bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO dynamic_data_log (id, bpan, previous_event_hash, event_hash, upload_type, record_hash, uploaded_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&log_id)
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind("SOH_UPDATE")
            .bind(&HashChainService::compute_hash(&format!("{}|{:.2}|{}|{}", bpan, state_of_health, total_cycles, degradation_class)))
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Audit log
        let audit_id = Uuid::new_v4();
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&actor_id)
            .bind("UPDATE_SOH")
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        // Log warnings for concerning SoH levels
        match &evaluation {
            SohEvaluation::ReuseCandidate => {
                warn!("BPAN {} SoH at {:.1}% — flagged as reuse candidate", bpan, state_of_health);
            }
            SohEvaluation::DegradedRecycleRecommended => {
                warn!("BPAN {} SoH at {:.1}% — recycling recommended", bpan, state_of_health);
            }
            SohEvaluation::EndOfLife => {
                warn!("BPAN {} SoH at {:.1}% — END OF LIFE reached", bpan, state_of_health);
            }
            _ => {}
        }

        info!("SoH updated for BPAN {}: {:.1}%, {} cycles, class {}", bpan, state_of_health, total_cycles, degradation_class);
        Ok(evaluation)
    }

    /// Ingest a telemetry data point from BMS.
    /// The payload is encrypted before storage for zero-knowledge compliance.
    #[instrument(name = "ingest_telemetry", skip(self, payload_json))]
    pub async fn ingest_telemetry(
        &self,
        bpan: &str,
        payload_json: &str,
        actor_id: Uuid,
    ) -> BpaResult<Uuid> {
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();

        // Encrypt the telemetry payload
        let encrypted_payload = self.encryption.encrypt(payload_json)?;

        let mut tx = self.pool.begin().await?;

        sqlx::query("INSERT INTO telemetry (id, bpan, cipher_algorithm, cipher_version, encrypted_payload, recorded_at) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(&id)
            .bind(bpan)
            .bind("AES-256-GCM")
            .bind(1i32)
            .bind(&encrypted_payload)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert telemetry: {:?}", e);
                BpaError::Database(e)
            })?;

        // Log as dynamic data event
        let log_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let event_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "INGEST_TELEMETRY",
            bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO dynamic_data_log (id, bpan, previous_event_hash, event_hash, upload_type, record_hash, uploaded_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&log_id)
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind("TELEMETRY")
            .bind(&HashChainService::compute_hash(payload_json))
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Telemetry ingested for BPAN: {}", bpan);
        Ok(id)
    }

    /// Record a thermal event (overheating, thermal runaway warning, etc.).
    #[instrument(name = "record_thermal_event", skip(self))]
    pub async fn record_thermal_event(
        &self,
        bpan: &str,
        event_type: &str,
        temperature_celsius: f64,
        details: &str,
        actor_id: Uuid,
    ) -> BpaResult<Uuid> {
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();

        // Thermal events are also logged as telemetry with encryption
        let payload = serde_json::json!({
            "event_type": event_type,
            "temperature_celsius": temperature_celsius,
            "details": details,
            "timestamp": now.to_string(),
        });

        let encrypted_payload = self.encryption.encrypt(&payload.to_string())?;

        let mut tx = self.pool.begin().await?;

        sqlx::query("INSERT INTO telemetry (id, bpan, cipher_algorithm, cipher_version, encrypted_payload, recorded_at) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(&id)
            .bind(bpan)
            .bind("AES-256-GCM")
            .bind(1i32)
            .bind(&encrypted_payload)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Log as dynamic data event
        let log_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let event_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "THERMAL_EVENT",
            bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO dynamic_data_log (id, bpan, previous_event_hash, event_hash, upload_type, record_hash, uploaded_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&log_id)
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind("THERMAL_EVENT")
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Also create a compliance alert for thermal events
        let alert_id = Uuid::new_v4();
        let severity = if temperature_celsius > 80.0 { "CRITICAL" } else { "WARNING" };
        let alert_msg = format!("Thermal event on BPAN {}: {}°C - {}", bpan, temperature_celsius, event_type);
        let encrypted_msg = self.encryption.encrypt(&alert_msg)?;

        sqlx::query("INSERT INTO alerts (id, cipher_algorithm, cipher_version, severity_hash, message_cipher, triggered_at, resolved) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&alert_id)
            .bind("AES-256-GCM")
            .bind(1i32)
            .bind(&HashChainService::compute_hash(severity))
            .bind(&encrypted_msg)
            .bind(&now)
            .bind(false)
            .execute(&mut *tx)
            .await?;

        // Audit log
        let audit_id = Uuid::new_v4();
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&actor_id)
            .bind("THERMAL_EVENT")
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        warn!("Thermal event recorded for BPAN {}: {}°C ({})", bpan, temperature_celsius, event_type);
        Ok(id)
    }

    /// Get the latest health status for a battery.
    pub async fn get_health(&self, bpan: &str) -> BpaResult<HealthStatus> {
        let result: Option<(f64, i32, String, bool, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT state_of_health, total_cycles, degradation_class, end_of_life, updated_at FROM battery_health WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await?;

        result
            .map(|(soh, cycles, class, eol, updated)| HealthStatus {
                bpan: bpan.to_string(),
                state_of_health: soh,
                total_cycles: cycles,
                degradation_class: class,
                end_of_life: eol,
                updated_at: updated,
            })
            .ok_or_else(|| BpaError::NotFound(format!("No health record for BPAN: {}", bpan)))
    }
}

/// Current health status of a battery.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub bpan: String,
    pub state_of_health: f64,
    pub total_cycles: i32,
    pub degradation_class: String,
    pub end_of_life: bool,
    pub updated_at: chrono::NaiveDateTime,
}
