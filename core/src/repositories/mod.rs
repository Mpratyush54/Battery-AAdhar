//! repositories/ — trait layer for all data access
//!
//! Each repository handles one entity group.
//! Concrete implementations (sqlx) land on Day 7–8.

pub mod audit_repo;
pub mod battery_repo;
pub mod key_repo;
pub mod lifecycle_repo;
pub mod material_repo;
pub mod ownership_repo;
pub mod stakeholder_repo;
pub mod battery_descriptor_repo;

pub use audit_repo::{AuditLogEntry, AuditRepositoryImpl};
pub use battery_repo::{BatteryRepository, BatteryRepositoryImpl, RepositoryError};
pub use key_repo::{KeyRepository, KeyRepositoryImpl};
pub use lifecycle_repo::{LifecycleRepositoryImpl, OwnershipRecord, RecyclingRecord, ReuseRecord};
pub use material_repo::MaterialRepository;
pub use ownership_repo::OwnershipRepositoryImpl;
pub use stakeholder_repo::StakeholderRepository;
pub use battery_descriptor_repo::*;

pub mod carbon_repo;
pub use carbon_repo::{CarbonRepository, CarbonRepositoryImpl, RepositoryError as CarbonRepoError};
pub mod dynamic_data_repo;
pub mod health_repo;
pub use dynamic_data_repo::DynamicDataRepositoryImpl;
pub use health_repo::HealthRepositoryImpl;

pub mod reuse_repo;
pub mod recycling_repo;
pub use reuse_repo::{ReuseRepository, ReuseRepositoryImpl};
pub use recycling_repo::{RecyclingRepository, RecyclingRepositoryImpl, CircularEconomyMetrics};
pub mod compliance_repo;
pub use compliance_repo::{ComplianceRepositoryImpl, ComplianceStats};
pub mod manufacturer_repo;
pub use manufacturer_repo::{ManufacturerRepository, ManufacturerRepositoryImpl};
