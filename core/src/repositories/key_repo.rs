//! key_repo.rs — KMS access (root keys, KEK, DEK, key rotation)

use async_trait::async_trait;
use super::battery_repo::RepositoryError;

#[async_trait]
pub trait KeyRepository: Send + Sync {
    /// Create or retrieve the current KEK.
    async fn get_current_kek(&self) -> Result<Vec<u8>, RepositoryError>;

    /// Create a new DEK for a BPAN.
    async fn create_dek(
        &self,
        bpan: &str,
        encrypted_dek: &[u8],
        kek_version: i32,
    ) -> Result<(), RepositoryError>;

    /// Retrieve wrapped DEK for a BPAN.
    async fn get_dek(&self, bpan: &str) -> Result<Option<Vec<u8>>, RepositoryError>;

    /// Rotate a DEK (create new version, re-encrypt data).
    async fn rotate_dek(
        &self,
        bpan: &str,
        new_encrypted_dek: &[u8],
        new_kek_version: i32,
    ) -> Result<(), RepositoryError>;

    /// Destroy a DEK (EOL battery).
    async fn destroy_dek(&self, bpan: &str) -> Result<(), RepositoryError>;

    /// Get the current KEK version.
    async fn get_kek_version(&self) -> Result<i32, RepositoryError>;

    /// Log a key rotation event.
    async fn log_key_rotation(
        &self,
        key_type: &str,
        old_version: i32,
        new_version: i32,
        rotated_by: &str,
    ) -> Result<(), RepositoryError>;
}
