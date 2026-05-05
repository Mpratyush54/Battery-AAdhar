//! key_repo.rs — Key management persistence
//!
//! Handles storage and retrieval of root keys, KEKs, and DEKs.

use super::battery_repo::RepositoryError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct KeyRepositoryImpl {
    pool: PgPool,
}

impl KeyRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        KeyRepositoryImpl { pool }
    }
}

#[async_trait]
pub trait KeyRepository: Send + Sync {
    async fn get_current_kek(&self) -> Result<Vec<u8>, RepositoryError>;
    async fn create_dek(
        &self,
        bpan: &str,
        encrypted_dek: &[u8],
        kek_version: i32,
    ) -> Result<(), RepositoryError>;
    async fn get_dek(&self, bpan: &str) -> Result<Option<Vec<u8>>, RepositoryError>;
    async fn rotate_dek(
        &self,
        bpan: &str,
        new_encrypted_dek: &[u8],
        new_kek_version: i32,
    ) -> Result<(), RepositoryError>;
    async fn destroy_dek(&self, bpan: &str) -> Result<(), RepositoryError>;
    async fn get_kek_version(&self) -> Result<i32, RepositoryError>;
    async fn log_key_rotation(
        &self,
        key_type: &str,
        old_version: i32,
        new_version: i32,
        rotated_by: &str,
    ) -> Result<(), RepositoryError>;
    async fn store_root_key(
        &self,
        encrypted_key: &[u8],
        hardware_backed: bool,
    ) -> Result<Uuid, RepositoryError>;
    async fn get_active_root_key(&self) -> Result<Option<Vec<u8>>, RepositoryError>;
    async fn store_kek(
        &self,
        encrypted_kek: &[u8],
        version: i32,
        root_key_id: Uuid,
    ) -> Result<Uuid, RepositoryError>;
}

#[async_trait]
impl KeyRepository for KeyRepositoryImpl {
    async fn store_root_key(
        &self,
        encrypted_key: &[u8],
        hardware_backed: bool,
    ) -> Result<Uuid, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO root_keys (id, encrypted_key, hardware_backed, status, created_at)
            VALUES ($1, $2, $3, 'active', $4)
            "#,
        )
        .bind(id)
        .bind(encrypted_key)
        .bind(hardware_backed)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id)
    }

    async fn get_active_root_key(&self) -> Result<Option<Vec<u8>>, RepositoryError> {
        let key = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT encrypted_key FROM root_keys WHERE status = 'active' ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(key)
    }

    async fn store_kek(
        &self,
        encrypted_kek: &[u8],
        version: i32,
        root_key_id: Uuid,
    ) -> Result<Uuid, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO kek_keys (id, encrypted_kek, version, root_key_id, status, created_at)
            VALUES ($1, $2, $3, $4, 'active', $5)
            "#,
        )
        .bind(id)
        .bind(encrypted_kek)
        .bind(version)
        .bind(root_key_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id)
    }

    async fn get_current_kek(&self) -> Result<Vec<u8>, RepositoryError> {
        let kek = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT encrypted_kek FROM kek_keys WHERE status = 'active' ORDER BY version DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .ok_or(RepositoryError::NotFound("no active KEK found".to_string()))?;

        Ok(kek)
    }

    async fn create_dek(
        &self,
        bpan: &str,
        encrypted_dek: &[u8],
        kek_version: i32,
    ) -> Result<(), RepositoryError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO battery_keys (bpan, encrypted_dek, kek_version, cipher_algorithm, cipher_version, key_status, created_at)
            VALUES ($1, $2, $3, 'AES-256-GCM', 1, 'active', $4)
            "#,
        )
        .bind(bpan)
        .bind(encrypted_dek)
        .bind(kek_version)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") {
                RepositoryError::AlreadyExists(format!("DEK for {} already exists", bpan))
            } else {
                RepositoryError::DatabaseError(e.to_string())
            }
        })?;

        Ok(())
    }

    async fn get_dek(&self, bpan: &str) -> Result<Option<Vec<u8>>, RepositoryError> {
        let dek = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT encrypted_dek FROM battery_keys WHERE bpan = $1 AND key_status = 'active'",
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(dek)
    }

    async fn rotate_dek(
        &self,
        bpan: &str,
        new_encrypted_dek: &[u8],
        new_kek_version: i32,
    ) -> Result<(), RepositoryError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE battery_keys
            SET encrypted_dek = $1, kek_version = $2, rotated_at = $3
            WHERE bpan = $4 AND key_status = 'active'
            "#,
        )
        .bind(new_encrypted_dek)
        .bind(new_kek_version)
        .bind(now)
        .bind(bpan)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn destroy_dek(&self, bpan: &str) -> Result<(), RepositoryError> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE battery_keys SET key_status = 'destroyed', rotated_at = $1 WHERE bpan = $2",
        )
        .bind(now)
        .bind(bpan)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_kek_version(&self) -> Result<i32, RepositoryError> {
        let version = sqlx::query_scalar::<_, i32>(
            "SELECT version FROM kek_keys WHERE status = 'active' ORDER BY version DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .unwrap_or(1); // Default to version 1 if none found

        Ok(version)
    }

    async fn log_key_rotation(
        &self,
        key_type: &str,
        old_version: i32,
        new_version: i32,
        rotated_by: &str,
    ) -> Result<(), RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO key_rotation_log (id, key_type, old_version, new_version, rotated_by, rotated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(key_type)
        .bind(old_version)
        .bind(new_version)
        .bind(rotated_by)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
