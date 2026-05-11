//! recycling_repo.rs — Recycling certification and circular economy metrics

use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use async_trait::async_trait;
use super::battery_repo::RepositoryError;

#[async_trait]
pub trait RecyclingRepository: Send + Sync {
    async fn insert_recycling(
        &self,
        bpan: &str,
        recycled_by: &str,
        method: &str,
        weight_kg: f32,
        standard: &str,
        li_percent: f32,
        co_percent: f32,
        ni_percent: f32,
        cert_hash: &str,
    ) -> Result<String, RepositoryError>;

    async fn get_metrics_by_manufacturer(
        &self,
        manufacturer_id: &str,
    ) -> Result<CircularEconomyMetrics, RepositoryError>;

    async fn get_metrics_by_chemistry(
        &self,
        chemistry_type: &str,
    ) -> Result<CircularEconomyMetrics, RepositoryError>;
}

pub struct RecyclingRepositoryImpl {
    pool: PgPool,
}

impl RecyclingRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        RecyclingRepositoryImpl { pool }
    }
}

#[async_trait]
impl RecyclingRepository for RecyclingRepositoryImpl {
    /// Record recycling with material recovery rates
    async fn insert_recycling(
        &self,
        bpan: &str,
        recycled_by: &str,
        method: &str,
        weight_kg: f32,
        standard: &str,
        li_percent: f32,
        co_percent: f32,
        ni_percent: f32,
        cert_hash: &str,
    ) -> Result<String, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO recycling_certifications
            (id, bpan, recycled_by, recycling_method, weight_processed_kg,
             li_recovery_percent, co_recovery_percent, ni_recovery_percent,
             certifying_standard, certification_hash, recycled_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(recycled_by)
        .bind(method)
        .bind(weight_kg)
        .bind(li_percent)
        .bind(co_percent)
        .bind(ni_percent)
        .bind(standard)
        .bind(cert_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id.to_string())
    }

    /// Get circular economy metrics by manufacturer
    async fn get_metrics_by_manufacturer(
        &self,
        manufacturer_id: &str,
    ) -> Result<CircularEconomyMetrics, RepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(DISTINCT rc.bpan) as battery_count,
                   AVG(rc.li_recovery_percent) as avg_li_recovery,
                   AVG(rc.co_recovery_percent) as avg_co_recovery,
                   AVG(rc.ni_recovery_percent) as avg_ni_recovery,
                   SUM(rc.weight_processed_kg) as total_weight_kg
            FROM recycling_certifications rc
            JOIN batteries b ON rc.bpan = b.bpan
            WHERE b.manufacturer_id = $1
            "#,
        )
        .bind(uuid::Uuid::parse_str(manufacturer_id).map_err(|e| RepositoryError::DatabaseError(e.to_string()))?)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| RepositoryError::DatabaseError(e.to_string()))?;

        use sqlx::Row;
        Ok(CircularEconomyMetrics {
            battery_count: row.try_get::<Option<i64>, _>("battery_count").unwrap_or(Some(0)).unwrap_or(0) as u32,
            avg_li_recovery: row.try_get::<Option<f64>, _>("avg_li_recovery").unwrap_or(Some(0.0)).unwrap_or(0.0) as f32,
            avg_co_recovery: row.try_get::<Option<f64>, _>("avg_co_recovery").unwrap_or(Some(0.0)).unwrap_or(0.0) as f32,
            avg_ni_recovery: row.try_get::<Option<f64>, _>("avg_ni_recovery").unwrap_or(Some(0.0)).unwrap_or(0.0) as f32,
            total_weight_processed_kg: row.try_get::<Option<f64>, _>("total_weight_kg").unwrap_or(Some(0.0)).unwrap_or(0.0) as f32,
        })
    }

    /// Get metrics by chemistry type
    async fn get_metrics_by_chemistry(
        &self,
        chemistry_type: &str,
    ) -> Result<CircularEconomyMetrics, RepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(DISTINCT rc.bpan) as battery_count,
                   AVG(rc.li_recovery_percent) as avg_li_recovery,
                   AVG(rc.co_recovery_percent) as avg_co_recovery,
                   AVG(rc.ni_recovery_percent) as avg_ni_recovery,
                   SUM(rc.weight_processed_kg) as total_weight_kg
            FROM recycling_certifications rc
            JOIN battery_descriptors bd ON b.bpan = bd.bpan
            WHERE bd.chemistry_type = $1
            "#,
        )
        .bind(chemistry_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| RepositoryError::DatabaseError(e.to_string()))?;

        use sqlx::Row;
        Ok(CircularEconomyMetrics {
            battery_count: row.try_get::<Option<i64>, _>("battery_count").unwrap_or(Some(0)).unwrap_or(0) as u32,
            avg_li_recovery: row.try_get::<Option<f64>, _>("avg_li_recovery").unwrap_or(Some(0.0)).unwrap_or(0.0) as f32,
            avg_co_recovery: row.try_get::<Option<f64>, _>("avg_co_recovery").unwrap_or(Some(0.0)).unwrap_or(0.0) as f32,
            avg_ni_recovery: row.try_get::<Option<f64>, _>("avg_ni_recovery").unwrap_or(Some(0.0)).unwrap_or(0.0) as f32,
            total_weight_processed_kg: row.try_get::<Option<f64>, _>("total_weight_kg").unwrap_or(Some(0.0)).unwrap_or(0.0) as f32,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CircularEconomyMetrics {
    pub battery_count: u32,
    pub avg_li_recovery: f32,
    pub avg_co_recovery: f32,
    pub avg_ni_recovery: f32,
    pub total_weight_processed_kg: f32,
}
