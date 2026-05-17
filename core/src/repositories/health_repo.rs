//! health_repo.rs — Health record persistence

use super::battery_repo::RepositoryError;
use crate::models::HealthRecord;
use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct HealthRepositoryImpl {
    pool: Option<PgPool>,
}

impl HealthRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        HealthRepositoryImpl { pool: Some(pool) }
    }

    pub fn new_stub() -> Self {
        HealthRepositoryImpl { pool: None }
    }

    pub async fn insert_health_record(
        &self,
        record: &HealthRecord,
    ) -> Result<String, RepositoryError> {
        let pool = self.pool.as_ref().ok_or(RepositoryError::NotFound("stub mode".to_string()))?;
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
        .execute(pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id)
    }

    pub async fn get_latest_health(
        &self,
        bpan: &str,
    ) -> Result<Option<HealthRecord>, RepositoryError> {
        let pool = self.pool.as_ref().ok_or(RepositoryError::NotFound("stub mode".to_string()))?;
        let row = sqlx::query(
            r#"
            SELECT * FROM battery_health 
            WHERE bpan = $1 
            ORDER BY reported_at DESC LIMIT 1
            "#,
        )
        .bind(bpan)
        .fetch_optional(pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(r) => {
                let status_str: String = r.try_get("health_status").unwrap_or_default();
                let health_status = match status_str.as_str() {
                    "OPERATIONAL" => crate::models::HealthStatus::Operational,
                    "SECOND_LIFE" => crate::models::HealthStatus::SecondLife,
                    "EOL_PROCESS" => crate::models::HealthStatus::EolProcess,
                    "WASTE" => crate::models::HealthStatus::Waste,
                    _ => crate::models::HealthStatus::Unknown,
                };

                let id_str: String = r.try_get("id").unwrap_or_default();
                let id = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4());
                let bpan_str: String = r.try_get("bpan").unwrap_or_default();
                let soh: f32 = r.try_get("state_of_health_percent").unwrap_or(0.0);
                let cycle_count: i32 = r.try_get("cycle_count").unwrap_or(0);

                Ok(Some(HealthRecord {
                    id,
                    bpan: bpan_str,
                    state_of_health_percent: soh,
                    health_status,
                    cycle_count: cycle_count as u32,
                    degradation_rate_percent_per_cycle: 0.1, // Default
                    degradation_class: "normal".to_string(), // Default
                    min_temperature_celsius: r.try_get("min_temperature_celsius").unwrap_or(0.0),
                    max_temperature_celsius: r.try_get("max_temperature_celsius").unwrap_or(0.0),
                    average_temperature_celsius: 0.0, // Default
                    cell_voltage_min_mv: 0.0,         // Default
                    cell_voltage_max_mv: 0.0,         // Default
                    internal_resistance_mohm: r.try_get("internal_resistance_mohm").unwrap_or(0.0),
                    error_flags: r.try_get("error_flags").unwrap_or_default(),
                    is_healthy: r.try_get("is_healthy").unwrap_or(true),
                    reported_by: r.try_get("reported_by").unwrap_or_default(),
                    reported_at: r.try_get("reported_at").unwrap_or_else(|_| Utc::now()),
                    record_number: r.try_get::<i32, _>("record_number").unwrap_or(1) as u32,
                    zk_proof_soh_gt_80: r.try_get("zk_proof_operational").unwrap_or_default(),
                    zk_proof_soh_gte_60: r.try_get("zk_proof_second_life").unwrap_or_default(),
                    zk_proof_soh_gte_30: r.try_get("zk_proof_recyclable").unwrap_or_default(),
                    proofs_generated_at: r.try_get("proofs_generated_at").unwrap_or_default(),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_health_history(
        &self,
        bpan: &str,
        limit: i32,
    ) -> Result<Vec<HealthRecord>, RepositoryError> {
        let pool = self.pool.as_ref().ok_or(RepositoryError::NotFound("stub mode".to_string()))?;
        let rows = sqlx::query(
            r#"
            SELECT * FROM battery_health 
            WHERE bpan = $1 
            ORDER BY reported_at DESC LIMIT $2
            "#,
        )
        .bind(bpan)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let records: Vec<HealthRecord> = rows
            .into_iter()
            .filter_map(|r| {
                let status_str: String = r.try_get("health_status").ok()?;
                let health_status = match status_str.as_str() {
                    "OPERATIONAL" => crate::models::HealthStatus::Operational,
                    "SECOND_LIFE" => crate::models::HealthStatus::SecondLife,
                    "EOL_PROCESS" => crate::models::HealthStatus::EolProcess,
                    "WASTE" => crate::models::HealthStatus::Waste,
                    _ => crate::models::HealthStatus::Unknown,
                };

                let id_str: String = r.try_get("id").ok()?;
                let id = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4());

                Some(HealthRecord {
                    id,
                    bpan: r.try_get("bpan").unwrap_or_default(),
                    state_of_health_percent: r.try_get("state_of_health_percent").unwrap_or(0.0),
                    health_status,
                    cycle_count: r.try_get::<i32, _>("cycle_count").unwrap_or(0) as u32,
                    degradation_rate_percent_per_cycle: 0.1,
                    degradation_class: "normal".to_string(),
                    min_temperature_celsius: r.try_get("min_temperature_celsius").unwrap_or(0.0),
                    max_temperature_celsius: r.try_get("max_temperature_celsius").unwrap_or(0.0),
                    average_temperature_celsius: 0.0,
                    cell_voltage_min_mv: 0.0,
                    cell_voltage_max_mv: 0.0,
                    internal_resistance_mohm: r.try_get("internal_resistance_mohm").unwrap_or(0.0),
                    error_flags: r.try_get("error_flags").unwrap_or_default(),
                    is_healthy: r.try_get("is_healthy").unwrap_or(true),
                    reported_by: r.try_get("reported_by").unwrap_or_default(),
                    reported_at: r.try_get("reported_at").unwrap_or_else(|_| Utc::now()),
                    record_number: r.try_get::<i32, _>("record_number").unwrap_or(1) as u32,
                    zk_proof_soh_gt_80: r.try_get("zk_proof_operational").unwrap_or_default(),
                    zk_proof_soh_gte_60: r.try_get("zk_proof_second_life").unwrap_or_default(),
                    zk_proof_soh_gte_30: r.try_get("zk_proof_recyclable").unwrap_or_default(),
                    proofs_generated_at: r.try_get("proofs_generated_at").unwrap_or_default(),
                })
            })
            .collect();

        Ok(records)
    }

    pub async fn get_avg_soh_by_manufacturer(
        &self,
        manufacturer_id: &str,
    ) -> Result<f32, RepositoryError> {
        let pool = self.pool.as_ref().ok_or(RepositoryError::NotFound("stub mode".to_string()))?;
        let avg = sqlx::query_scalar::<_, Option<f32>>(
            r#"
            SELECT AVG(h.state_of_health_percent) 
            FROM battery_health h
            JOIN batteries b ON h.bpan = b.bpan
            WHERE b.manufacturer_id = $1
            "#,
        )
        .bind(manufacturer_id)
        .fetch_optional(pool)
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
        let pool = self.pool.as_ref().ok_or(RepositoryError::NotFound("stub mode".to_string()))?;
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
        .fetch_optional(pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .flatten()
        .ok_or(RepositoryError::NotFound("no health data".to_string()))?;

        Ok(avg)
    }

    pub async fn check_rate_limit(&self, bpan: &str) -> Result<bool, RepositoryError> {
        // Check if there was a health update in the last hour
        let pool = self.pool.as_ref().ok_or(RepositoryError::NotFound("stub mode".to_string()))?;
        let recent_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM battery_health 
            WHERE bpan = $1 AND reported_at > NOW() - INTERVAL '1 hour'
            "#,
        )
        .bind(bpan)
        .fetch_one(pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(recent_count > 0) // true = rate limited
    }
}
