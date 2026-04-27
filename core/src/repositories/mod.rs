//! repositories/ — trait layer for all data access
//!
//! Each repository handles one entity group.
//! Concrete implementations (sqlx) land on Day 7–8.

pub mod audit_repo;
pub mod battery_repo;
pub mod key_repo;
pub mod lifecycle_repo;
pub mod stakeholder_repo;

pub use audit_repo::{AuditLogEntry, AuditRepositoryImpl};
pub use battery_repo::{BatteryRepository, BatteryRepositoryImpl, RepositoryError};
pub use key_repo::{KeyRepository, KeyRepositoryImpl};
pub use lifecycle_repo::{LifecycleRepositoryImpl, OwnershipRecord, RecyclingRecord, ReuseRecord};
pub use stakeholder_repo::StakeholderRepository;
