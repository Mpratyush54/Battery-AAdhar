//! telemetry.rs — BMS telemetry ingestion and query service
//!
//! Handles encrypted telemetry data from battery management systems.
//! Supports batch ingestion and time-range queries.

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::encryption::EncryptionService;

#[derive(Debug)]
pub enum TelemetryError {
    NotFound(String),
    Unauthorized(String),
    InvalidData(String),
    DatabaseError(String),
    EncryptionError(String),
}

impl std::fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryError::NotFound(msg) => write!(f, "not found: {}", msg),
            TelemetryError::Unauthorized(msg) => write!(f, "unauthorized: {}", msg),
            TelemetryError::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            TelemetryError::DatabaseError(msg) => write!(f, "database error: {}", msg),
            TelemetryError::EncryptionError(msg) => write!(f, "encryption error: {}", msg),
        }
    }
}

impl std::error::Error for TelemetryError {}

#[derive(Debug, Clone)]
pub struct TelemetryRecord {
    pub id: Uuid,
    pub bpan: String,
    pub encrypted_payload: String,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
    pub recorded_at: chrono::DateTime<Utc>,
    pub ingested_at: chrono::DateTime<Utc>,
}

#[async_trait]
pub trait TelemetryService: Send + Sync {
    async fn ingest_telemetry(
        &self,
        bpan: &str,
        plaintext_payload: &[u8],
    ) -> Result<String, TelemetryError>;

    async fn ingest_batch(
        &self,
        bpan: &str,
        payloads: Vec<Vec<u8>>,
    ) -> Result<Vec<String>, TelemetryError>;

    async fn get_telemetry_range(
        &self,
        bpan: &str,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<TelemetryRecord>, TelemetryError>;
}

pub struct TelemetryServiceImpl {
    pool: PgPool,
    encryption: EncryptionService,
}

impl TelemetryServiceImpl {
    pub fn new(pool: PgPool, encryption: EncryptionService) -> Self {
        TelemetryServiceImpl { pool, encryption }
    }
}

#[async_trait]
impl TelemetryService for TelemetryServiceImpl {
    async fn ingest_telemetry(
        &self,
        bpan: &str,
        plaintext_payload: &[u8],
    ) -> Result<String, TelemetryError> {
        if bpan.len() != 21 {
            return Err(TelemetryError::InvalidData("invalid BPAN".to_string()));
        }

        if plaintext_payload.is_empty() {
            return Err(TelemetryError::InvalidData("empty payload".to_string()));
        }

        // Encrypt payload
        let encrypted = self
            .encryption
            .encrypt_bytes(plaintext_payload)
            .map_err(|e| TelemetryError::EncryptionError(e.to_string()))?;

        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO telemetry 
            (id, bpan, encrypted_payload, cipher_algorithm, cipher_version, recorded_at, ingested_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(encrypted)
        .bind("AES-256-GCM")
        .bind(1i32)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| TelemetryError::DatabaseError(e.to_string()))?;

        tracing::info!(bpan = %bpan, id = %id, "telemetry ingested");

        Ok(id.to_string())
    }

    async fn ingest_batch(
        &self,
        bpan: &str,
        payloads: Vec<Vec<u8>>,
    ) -> Result<Vec<String>, TelemetryError> {
        let mut ids = Vec::with_capacity(payloads.len());
        for payload in payloads {
            let id = self.ingest_telemetry(bpan, &payload).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    async fn get_telemetry_range(
        &self,
        bpan: &str,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<TelemetryRecord>, TelemetryError> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, i32, chrono::DateTime<Utc>, chrono::DateTime<Utc>)>(
            r#"
            SELECT id, bpan, encrypted_payload, cipher_algorithm, cipher_version, recorded_at, ingested_at
            FROM telemetry
            WHERE bpan = $1 AND recorded_at >= $2 AND recorded_at <= $3
            ORDER BY recorded_at ASC
            "#,
        )
        .bind(bpan)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TelemetryError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(id, bpan, encrypted_payload, cipher_algorithm, cipher_version, recorded_at, ingested_at)| {
                TelemetryRecord {
                    id,
                    bpan,
                    encrypted_payload,
                    cipher_algorithm,
                    cipher_version,
                    recorded_at,
                    ingested_at,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require real Postgres
}
