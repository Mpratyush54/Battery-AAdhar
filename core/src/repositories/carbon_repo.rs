//! carbon_repo.rs — Carbon footprint persistence with verification
//!
//! Stores BCF data + verification status + audit trail.

use super::battery_repo::RepositoryError;
use crate::models::CarbonFootprint;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct CarbonRepositoryImpl {
    pool: PgPool,
}

impl CarbonRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        CarbonRepositoryImpl { pool }
    }

    /// Store carbon footprint data
    pub async fn insert_carbon_footprint(
        &self,
        bpan: &str,
        cf: &CarbonFootprint,
        encrypted_blob: &[u8],
    ) -> Result<String, RepositoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO battery_carbon_footprint 
            (bpan, total_emissions_kg_co2e, emissions_per_kwh, carbon_hash, encrypted_blob, submitted_by, version, verified, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(bpan)
        .bind(cf.total_emissions_kg_co2e)
        .bind(cf.emissions_per_kwh)
        .bind(&cf.carbon_hash)
        .bind(encrypted_blob)
        .bind(&cf.submitted_by)
        .bind(cf.submitted_version)
        .bind(false) // Not verified yet
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // Log to submission log
        self.log_carbon_submission(bpan, &cf.submitted_by, 1, &cf.carbon_hash)
            .await?;

        Ok(id)
    }

    /// Mark carbon footprint as verified
    pub async fn verify_carbon_footprint(
        &self,
        bpan: &str,
        verified_by: &str,
        standard: &str,
    ) -> Result<(), RepositoryError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE battery_carbon_footprint 
            SET verified = true, verified_by = $1, verified_at = $2, verification_standard = $3
            WHERE bpan = $4
            "#,
        )
        .bind(verified_by)
        .bind(now)
        .bind(standard)
        .bind(bpan)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Retrieve carbon footprint
    pub async fn get_carbon_footprint(
        &self,
        bpan: &str,
    ) -> Result<
        Option<(
            f32,
            String,
            bool,
            Option<String>,
            Option<chrono::DateTime<Utc>>,
        )>,
        RepositoryError,
    > {
        use sqlx::Row;
        let row = sqlx::query(
            r#"
            SELECT total_emissions_kg_co2e, carbon_hash, verified, verified_by, verified_at
            FROM battery_carbon_footprint WHERE bpan = $1
            "#,
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| {
            (
                r.get("total_emissions_kg_co2e"),
                r.get("carbon_hash"),
                r.get("verified"),
                r.get("verified_by"),
                r.get("verified_at"),
            )
        }))
    }

    /// Check hash integrity (tamper detection)
    pub async fn check_hash_integrity(
        &self,
        bpan: &str,
        expected_hash: &str,
    ) -> Result<bool, RepositoryError> {
        let stored_hash = sqlx::query_scalar::<_, String>(
            "SELECT carbon_hash FROM battery_carbon_footprint WHERE bpan = $1",
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .ok_or(RepositoryError::NotFound(format!(
            "BPAN {} not found",
            bpan
        )))?;

        Ok(stored_hash == expected_hash)
    }

    /// Log carbon submission for audit
    async fn log_carbon_submission(
        &self,
        bpan: &str,
        submitted_by: &str,
        version: i32,
        carbon_hash: &str,
    ) -> Result<(), RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO static_data_submission_log 
            (id, bpan, submitted_by, data_type, version, encrypted_hash, submitted_at)
            VALUES ($1, $2, $3, 'BCF', $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(submitted_by)
        .bind(version)
        .bind(carbon_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get submission history
    pub async fn get_carbon_submission_history(
        &self,
        bpan: &str,
    ) -> Result<Vec<(String, String, i32, String)>, RepositoryError> {
        use sqlx::Row;
        let logs = sqlx::query(
            r#"
            SELECT id, submitted_by, version, encrypted_hash
            FROM static_data_submission_log 
            WHERE bpan = $1 AND data_type = 'BCF'
            ORDER BY submitted_at DESC
            "#,
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(logs
            .into_iter()
            .map(|r| {
                let id: uuid::Uuid = r.get("id");
                let version: i32 = r.get("version");
                (
                    id.to_string(),
                    r.get("submitted_by"),
                    version,
                    r.get("encrypted_hash"),
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests in Day 9 D1
}
