#[derive(sqlx::FromRow, Debug)]
pub struct Battery {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow, Debug)]
pub struct BatteryIdentifier {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow, Debug)]
pub struct BatteryDescriptor {
    pub id: uuid::Uuid,
    pub bpan: String,
    // Add more fields if needed, or leave stub
}

#[derive(sqlx::FromRow, Debug)]
pub struct Stakeholder {
    pub id: String,
}

pub mod lifecycle;
pub use lifecycle::{
    BatteryOwner, LifecycleEvent, LifecycleState, LifecycleTimeline, OwnershipTransfer,
};
pub mod carbon;
pub use carbon::{
    CarbonComparison, CarbonFootprint, CarbonFootprintPublic, CarbonFootprintRequest,
};
pub mod health;
pub use health::{HealthAggregate, HealthHistory, HealthRecord, HealthStatus, HealthUpdateRequest};
