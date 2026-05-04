//! dynamic_data_repo.rs — Dynamic data log with hash-chain integrity
//!
//! Like audit log, but for per-battery runtime data.
//! Each health update creates a hash-chained entry.

use super::battery_repo::RepositoryError;
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

pub struct DynamicDataRepositoryImpl {
    pool: PgPool,
}

impl DynamicDataRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        DynamicDataRepositoryImpl { pool }
    }

    pub async fn log_health_update(
        &self,
        bpan: &str,
        soh: f32,
        cycles: u32,
        temperature_avg: f32,
    ) -> Result<String, RepositoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Get previous entry's hash (for chain link)
        let prev_hash = sqlx::query_scalar::<_, String>(
            "SELECT entry_hash FROM dynamic_data_log WHERE bpan = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .unwrap_or_else(|| "0".to_string()); // Genesis hash

        // Compute hash for this entry
        let mut hasher = Sha256::new();
        hasher.update(soh.to_le_bytes());
        hasher.update(cycles.to_le_bytes());
        hasher.update(temperature_avg.to_le_bytes());
        hasher.update(prev_hash.as_bytes());
        let entry_hash = format!("{:x}", hasher.finalize());

        sqlx::query(
            r#"
            INSERT INTO dynamic_data_log 
            (id, bpan, data_type, soh_percent, cycle_count, temperature_avg, 
             entry_hash, entry_hash_prev, created_at)
            VALUES ($1, $2, 'HEALTH_UPDATE', $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&id)
        .bind(bpan)
        .bind(soh)
        .bind(cycles as i32)
        .bind(temperature_avg)
        .bind(&entry_hash)
        .bind(&prev_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id)
    }

    pub async fn verify_hash_chain(&self, bpan: &str) -> Result<bool, RepositoryError> {
        let entries = sqlx::query(
            "SELECT entry_hash, entry_hash_prev FROM dynamic_data_log WHERE bpan = $1 ORDER BY created_at ASC",
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // Verify chain integrity
        let mut prev_hash = "0".to_string();
        for entry in entries {
            let entry_hash: String = entry.get("entry_hash");
            let entry_hash_prev: String = entry.get("entry_hash_prev");

            if entry_hash_prev != prev_hash {
                return Ok(false); // Chain broken
            }
            prev_hash = entry_hash;
        }

        Ok(true) // Chain valid
    }
}
