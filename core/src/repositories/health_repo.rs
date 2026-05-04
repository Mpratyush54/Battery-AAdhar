//! health_repo.rs — Health record persistence

use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use crate::models::HealthRecord;
use super::battery_repo::RepositoryError;

pub struct HealthRepositoryImpl {
    pool: PgPool,
}

impl HealthRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        HealthRepositoryImpl { pool }
    }

    pub async fn insert_health_record(
        &self,
        record: &HealthRecord,
    ) -> Result<String, RepositoryError> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO battery_health 
            (id, bpan, state_of_health_percent, health_status, cycle_count, 
             min_temperature_celsius, max_temperature_celsius, internal_resistance_mohm,
             error_flags, is_healthy, zk_proof_operational, zk_proof_second_life, 
             zk_proof_recyclable, proofs_generated_at, reported_by, reported_at, record_number)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            "#,
        )
        .bind(&id)
        .bind(&record.bpan)
        .bind(record.state_of_health_percent)
        .bind(record.health_status.to_string())
        .bind(record.cycle_count as i32)
        .bind(record.min_temperature_celsius)
        .bind(record.max_temperature_celsius)
        .bind(record.internal_resistance_mohm)
        .bind(&record.error_flags)
        .bind(record.is_healthy)
        .bind(&record.zk_proof_soh_gt_80)
        .bind(&record.zk_proof_soh_gte_60)
        .bind(&record.zk_proof_soh_gte_30)
        .bind(record.proofs_generated_at)
        .bind(&record.reported_by)
        .bind(record.reported_at)
        .bind(record.record_number as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id)
    }

    pub async fn get_latest_health(
        &self,
        bpan: &str,
    ) -> Result<Option<HealthRecord>, RepositoryError> {
        // TODO: Fetch and deserialize
        Ok(None)
    }

    pub async fn get_health_history(
        &self,
        bpan: &str,
        limit: i32,
    ) -> Result<Vec<HealthRecord>, RepositoryError> {
        // TODO: Fetch ordered by reported_at DESC
        Ok(vec![])
    }

    pub async fn get_avg_soh_by_manufacturer(
        &self,
        manufacturer_id: &str,
    ) -> Result<f32, RepositoryError> {
        let avg = sqlx::query_scalar::<_, Option<f32>>(
            r#"
            SELECT AVG(h.state_of_health_percent) 
            FROM battery_health h
            JOIN batteries b ON h.bpan = b.bpan
            WHERE b.manufacturer_id = $1
            "#,
        )
        .bind(manufacturer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .flatten()
        .ok_or(RepositoryError::NotFound("no health data".to_string()))?;

        Ok(avg)
    }

    pub async fn get_avg_soh_by_chemistry(
        &self,
        chemistry_type: &str,
    ) -> Result<f32, RepositoryError> {
        let avg = sqlx::query_scalar::<_, Option<f32>>(
            r#"
            SELECT AVG(h.state_of_health_percent) 
            FROM battery_health h
            JOIN batteries b ON h.bpan = b.bpan
            JOIN battery_descriptor bd ON b.bpan = bd.bpan
            WHERE bd.chemistry_type = $1
            "#,
        )
        .bind(chemistry_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .flatten()
        .ok_or(RepositoryError::NotFound("no health data".to_string()))?;

        Ok(avg)
    }

    pub async fn check_rate_limit(
        &self,
        bpan: &str,
    ) -> Result<bool, RepositoryError> {
        // Check if there was a health update in the last hour
        let recent_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM battery_health 
            WHERE bpan = $1 AND reported_at > NOW() - INTERVAL '1 hour'
            "#,
        )
        .bind(bpan)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(recent_count > 0) // true = rate limited
    }
}
