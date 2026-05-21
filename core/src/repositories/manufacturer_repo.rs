use async_trait::async_trait;
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

use crate::models::Manufacturers;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManufacturerRepoError(pub String);

impl std::fmt::Display for ManufacturerRepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ManufacturerRepoError {}

#[async_trait]
pub trait ManufacturerRepository: Send + Sync {
    async fn insert(&self, mfr: &Manufacturers) -> Result<(), ManufacturerRepoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Manufacturers>, ManufacturerRepoError>;
    async fn find_by_code(
        &self,
        code: &str,
    ) -> Result<Option<Manufacturers>, ManufacturerRepoError>;
    async fn find_by_name(
        &self,
        name: &str,
    ) -> Result<Option<Manufacturers>, ManufacturerRepoError>;
    async fn list_all(&self) -> Result<Vec<Manufacturers>, ManufacturerRepoError>;
    async fn get_last_code(&self) -> Result<Option<String>, ManufacturerRepoError>;
}

pub struct ManufacturerRepositoryImpl {
    pool: Pool<Postgres>,
}

impl ManufacturerRepositoryImpl {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ManufacturerRepository for ManufacturerRepositoryImpl {
    async fn insert(&self, mfr: &Manufacturers) -> Result<(), ManufacturerRepoError> {
        sqlx::query(
            "INSERT INTO manufacturers (id, manufacturer_code, name, country_code, encrypted_profile, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(mfr.id)
        .bind(&mfr.manufacturer_code)
        .bind(&mfr.name)
        .bind(&mfr.country_code)
        .bind(&mfr.encrypted_profile)
        .bind(mfr.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ManufacturerRepoError(e.to_string()))?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Manufacturers>, ManufacturerRepoError> {
        let row: Result<Option<Manufacturers>, sqlx::Error> = sqlx::query_as::<_, Manufacturers>(
            "SELECT id, manufacturer_code, name, country_code, encrypted_profile, created_at FROM manufacturers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await;
        row.map_err(|e| ManufacturerRepoError(e.to_string()))
    }

    async fn find_by_code(
        &self,
        code: &str,
    ) -> Result<Option<Manufacturers>, ManufacturerRepoError> {
        let row: Result<Option<Manufacturers>, sqlx::Error> = sqlx::query_as::<_, Manufacturers>(
            "SELECT id, manufacturer_code, name, country_code, encrypted_profile, created_at FROM manufacturers WHERE manufacturer_code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await;
        row.map_err(|e| ManufacturerRepoError(e.to_string()))
    }

    async fn find_by_name(
        &self,
        name: &str,
    ) -> Result<Option<Manufacturers>, ManufacturerRepoError> {
        let row: Result<Option<Manufacturers>, sqlx::Error> = sqlx::query_as::<_, Manufacturers>(
            "SELECT id, manufacturer_code, name, country_code, encrypted_profile, created_at FROM manufacturers WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await;
        row.map_err(|e| ManufacturerRepoError(e.to_string()))
    }

    async fn list_all(&self) -> Result<Vec<Manufacturers>, ManufacturerRepoError> {
        let rows: Result<Vec<Manufacturers>, sqlx::Error> = sqlx::query_as::<_, Manufacturers>(
            "SELECT id, manufacturer_code, name, country_code, encrypted_profile, created_at FROM manufacturers ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await;
        rows.map_err(|e| ManufacturerRepoError(e.to_string()))
    }

    async fn get_last_code(&self) -> Result<Option<String>, ManufacturerRepoError> {
        let row = sqlx::query(
            "SELECT manufacturer_code FROM manufacturers ORDER BY manufacturer_code DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ManufacturerRepoError(e.to_string()))?;
        Ok(row.map(|r| r.get("manufacturer_code")))
    }
}
