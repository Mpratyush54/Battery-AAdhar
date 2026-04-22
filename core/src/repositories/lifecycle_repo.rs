//! lifecycle_repo.rs — Battery lifecycle state transitions & recycling records

use async_trait::async_trait;
use super::battery_repo::RepositoryError;

#[async_trait]
pub trait LifecycleRepository: Send + Sync {
    /// Record ownership transfer.
    async fn log_ownership_transfer(
        &self,
        bpan: &str,
        from_owner_hash: &str,
        to_owner_hash: &str,
        reason: &str,
    ) -> Result<(), RepositoryError>;

    /// Record second-life certification.
    async fn log_reuse_certification(
        &self,
        bpan: &str,
        reuse_type: &str,
        certifier: &str,
    ) -> Result<(), RepositoryError>;

    /// Get ownership history for a battery.
    async fn get_ownership_history(
        &self,
        bpan: &str,
    ) -> Result<Vec<OwnershipRecord>, RepositoryError>;

    /// Record recycling completion.
    async fn log_recycling(
        &self,
        bpan: &str,
        recycler_name: &str,
        recovered_percentage: f32,
    ) -> Result<(), RepositoryError>;

    /// Get recycling records for a battery.
    async fn get_recycling_records(
        &self,
        bpan: &str,
    ) -> Result<Vec<RecyclingRecord>, RepositoryError>;

    /// Get the current battery status (Operational / Second Life / EOL / Waste).
    async fn get_battery_status(&self, bpan: &str) -> Result<Option<String>, RepositoryError>;

    /// Update battery status based on SoH or manual override.
    async fn update_battery_status(
        &self,
        bpan: &str,
        new_status: &str,
    ) -> Result<(), RepositoryError>;
}

pub struct OwnershipRecord {
    pub id: String,
    pub bpan: String,
    pub owner_hash: String,
    pub start_time: String,
    pub end_time: Option<String>,
}

pub struct RecyclingRecord {
    pub id: String,
    pub bpan: String,
    pub recycler_name: String,
    pub recovered_percentage: f32,
    pub recycled_at: String,
}
