//! reuse_repo.rs — Second-life certification persistence

use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use async_trait::async_trait;
// use crate::models::HealthStatus;
use super::battery_repo::RepositoryError;

#[async_trait]
pub trait ReuseRepository: Send + Sync {
    async fn insert_certification(
        &self,
        bpan: &str,
        soh: f32,
        certified_by: &str,
        application: &str,
        expected_years: i32,
        cert_hash: &str,
    ) -> Result<String, RepositoryError>;

    async fn get_certifications(
        &self,
        bpan: &str,
    ) -> Result<Vec<(String, f32, String, String)>, RepositoryError>;
}

pub struct ReuseRepositoryImpl {
    pool: PgPool,
}

impl ReuseRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        ReuseRepositoryImpl { pool }
    }
}

#[async_trait]
impl ReuseRepository for ReuseRepositoryImpl {
    /// Record reuse certification
    async fn insert_certification(
        &self,
        bpan: &str,
        soh: f32,
        certified_by: &str,
        application: &str,
        expected_years: i32,
        cert_hash: &str,
    ) -> Result<String, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO reuse_certifications
            (id, bpan, soh_at_certification, certified_by, intended_application,
             expected_second_life_years, certification_hash, certified_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(soh)
        .bind(certified_by)
        .bind(application)
        .bind(expected_years)
        .bind(cert_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e: sqlx::Error| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id.to_string())
    }

    /// Get reuse certificates for battery
    async fn get_certifications(
        &self,
        bpan: &str,
    ) -> Result<Vec<(String, f32, String, String)>, RepositoryError> {
        // Returns: (cert_id, soh, application, certified_at)
        let certs = sqlx::query(
            "SELECT id, soh_at_certification, intended_application, certified_at FROM reuse_certifications WHERE bpan = $1 ORDER BY certified_at DESC"
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| RepositoryError::DatabaseError(e.to_string()))?;

        use sqlx::Row;
        Ok(certs
            .into_iter()
            .map(|c| {
                (
                    c.try_get::<uuid::Uuid, _>("id").unwrap_or_default().to_string(),
                    c.try_get::<f64, _>("soh_at_certification").unwrap_or(0.0) as f32,
                    c.try_get::<String, _>("intended_application").unwrap_or_default(),
                    c.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("certified_at").unwrap_or(None).map(|t| t.to_string()).unwrap_or_default(),
                )
            })
            .collect())
    }
}
