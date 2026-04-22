//! battery_repo.rs — Battery & descriptor data access
//!
//! Handles:
//!   batteries (main BPAN table)
//!   battery_identifiers (BI)
//!   battery_descriptor (BDS)
//!   battery_material_composition (BMCS)
//!   battery_health (dynamic SoH)
//!   carbon_footprint (BCF)

use crate::models::{Battery, BatteryIdentifier, BatteryDescriptor};
use async_trait::async_trait;

#[async_trait]
pub trait BatteryRepository: Send + Sync {
    /// Create a new battery with the given BPAN.
    async fn create_battery(&self, bpan: &str) -> Result<Battery, RepositoryError>;

    /// Retrieve battery by BPAN.
    async fn get_battery_by_bpan(&self, bpan: &str) -> Result<Option<Battery>, RepositoryError>;

    /// List all batteries (with optional pagination).
    async fn list_batteries(&self, limit: i32, offset: i32) -> Result<Vec<Battery>, RepositoryError>;

    /// Update battery status/health data.
    async fn update_battery_status(&self, bpan: &str, soh: f32) -> Result<(), RepositoryError>;

    /// Create or update battery identifier (BI).
    async fn upsert_battery_identifier(&self, bi: &BatteryIdentifier) -> Result<(), RepositoryError>;

    /// Retrieve battery descriptor (BDS) by BPAN.
    async fn get_battery_descriptor(&self, bpan: &str) -> Result<Option<BatteryDescriptor>, RepositoryError>;

    /// Get current State of Health for a battery.
    async fn get_soh(&self, bpan: &str) -> Result<Option<f32>, RepositoryError>;

    /// Update SoH and trigger status transition if threshold crossed.
    async fn update_soh(&self, bpan: &str, new_soh: f32) -> Result<(), RepositoryError>;
}

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
            RepositoryError::NotFound(msg)       => write!(f, "not found: {msg}"),
            RepositoryError::AlreadyExists(msg)  => write!(f, "already exists: {msg}"),
            RepositoryError::DatabaseError(msg)  => write!(f, "database error: {msg}"),
            RepositoryError::ValidationError(msg) => write!(f, "validation error: {msg}"),
        }
    }
}

impl std::error::Error for RepositoryError {}
