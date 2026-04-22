//! stakeholder_repo.rs — Manufacturer, recycler, government access control

use crate::models::Stakeholder;
use async_trait::async_trait;
use super::battery_repo::RepositoryError;

#[async_trait]
pub trait StakeholderRepository: Send + Sync {
    /// Create a new stakeholder (manufacturer, recycler, etc).
    async fn create_stakeholder(
        &self,
        role: &str,
        encrypted_profile: &str,
    ) -> Result<Stakeholder, RepositoryError>;

    /// Get stakeholder by ID.
    async fn get_stakeholder(&self, id: &str) -> Result<Option<Stakeholder>, RepositoryError>;

    /// List stakeholders by role.
    async fn list_stakeholders_by_role(
        &self,
        role: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Stakeholder>, RepositoryError>;

    /// Update stakeholder profile.
    async fn update_stakeholder_profile(
        &self,
        id: &str,
        encrypted_profile: &str,
    ) -> Result<(), RepositoryError>;

    /// Check if stakeholder has access to a battery's data (RBAC).
    async fn has_access(
        &self,
        stakeholder_id: &str,
        bpan: &str,
        access_level: &str,
    ) -> Result<bool, RepositoryError>;
}
