//! repositories/ — trait layer for all data access
//!
//! Each repository handles one entity group.
//! Concrete implementations (sqlx) land on Day 7–8.

pub mod battery_repo;
pub mod stakeholder_repo;
pub mod key_repo;
pub mod audit_repo;
pub mod lifecycle_repo;

pub use battery_repo::BatteryRepository;
pub use stakeholder_repo::StakeholderRepository;
pub use key_repo::KeyRepository;
pub use audit_repo::AuditRepository;
pub use lifecycle_repo::LifecycleRepository;
