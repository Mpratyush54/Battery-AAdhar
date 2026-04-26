//! battery_repo.rs — Battery data persistence (PostgreSQL)
//!
//! Concrete implementation of the BatteryRepository trait from Day 2.
//! Uses sqlx for parameterized queries to prevent SQL injection.

use crate::models::{Battery, BatteryIdentifier, BatteryDescriptor};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug)]
pub enum RepositoryError {
    NotFound(String),
    AlreadyExists(String),
    DatabaseError(String),
    ValidationError(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::NotFound(msg) => write!(f, "not found: {}", msg),
            RepositoryError::AlreadyExists(msg) => write!(f, "already exists: {}", msg),
            RepositoryError::DatabaseError(msg) => write!(f, "database error: {}", msg),
            RepositoryError::ValidationError(msg) => write!(f, "validation error: {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

/// The concrete BatteryRepository implementation
pub struct BatteryRepositoryImpl {
    pool: PgPool,
}

impl BatteryRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        BatteryRepositoryImpl { pool }
    }
}

#[async_trait]
pub trait BatteryRepository: Send + Sync {
    async fn create_battery(&self, bpan: &str) -> Result<Battery, RepositoryError>;
    async fn get_battery_by_bpan(&self, bpan: &str) -> Result<Option<Battery>, RepositoryError>;
    async fn list_batteries(&self, limit: i32, offset: i32) -> Result<Vec<Battery>, RepositoryError>;
    async fn update_battery_status(&self, bpan: &str, soh: f32) -> Result<(), RepositoryError>;
    async fn upsert_battery_identifier(&self, bi: &BatteryIdentifier) -> Result<(), RepositoryError>;
    async fn get_battery_descriptor(&self, bpan: &str) -> Result<Option<BatteryDescriptor>, RepositoryError>;
    async fn get_soh(&self, bpan: &str) -> Result<Option<f32>, RepositoryError>;
    async fn update_soh(&self, bpan: &str, new_soh: f32) -> Result<(), RepositoryError>;
}

#[async_trait]
impl BatteryRepository for BatteryRepositoryImpl {
    async fn create_battery(&self, bpan: &str) -> Result<Battery, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let battery = sqlx::query_as::<_, Battery>(
            r#"
            INSERT INTO batteries (id, bpan, created_at, updated_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, bpan, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") {
                RepositoryError::AlreadyExists(format!("BPAN {} already exists", bpan))
            } else {
                RepositoryError::DatabaseError(e.to_string())
            }
        })?;

        Ok(battery)
    }

    async fn get_battery_by_bpan(&self, bpan: &str) -> Result<Option<Battery>, RepositoryError> {
        let battery = sqlx::query_as::<_, Battery>(
            "SELECT id, bpan, created_at, updated_at FROM batteries WHERE bpan = $1",
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(battery)
    }

    async fn list_batteries(&self, limit: i32, offset: i32) -> Result<Vec<Battery>, RepositoryError> {
        let batteries = sqlx::query_as::<_, Battery>(
            "SELECT id, bpan, created_at, updated_at FROM batteries LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(batteries)
    }

    async fn update_battery_status(&self, bpan: &str, soh: f32) -> Result<(), RepositoryError> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE batteries SET updated_at = $1 WHERE bpan = $2",
        )
        .bind(now)
        .bind(bpan)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound(format!("BPAN {} not found", bpan)));
        }

        Ok(())
    }

    async fn upsert_battery_identifier(
        &self,
        bi: &BatteryIdentifier,
    ) -> Result<(), RepositoryError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO battery_identifiers (id, bpan, created_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (bpan) DO UPDATE SET created_at = $3
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&bi.bpan)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_battery_descriptor(
        &self,
        bpan: &str,
    ) -> Result<Option<BatteryDescriptor>, RepositoryError> {
        let descriptor = sqlx::query_as::<_, BatteryDescriptor>(
            "SELECT * FROM battery_descriptor WHERE bpan = $1",
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(descriptor)
    }

    async fn get_soh(&self, bpan: &str) -> Result<Option<f32>, RepositoryError> {
        let row = sqlx::query_scalar::<_, f32>(
            "SELECT state_of_health FROM battery_health WHERE bpan = $1 ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(row)
    }

    async fn update_soh(&self, bpan: &str, new_soh: f32) -> Result<(), RepositoryError> {
        if new_soh < 0.0 || new_soh > 100.0 {
            return Err(RepositoryError::ValidationError(
                format!("SoH must be between 0 and 100, got {}", new_soh),
            ));
        }

        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO battery_health (id, bpan, state_of_health, updated_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(new_soh)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
