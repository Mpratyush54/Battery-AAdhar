use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DataAccessControl {
    pub id: uuid::Uuid,
    pub stakeholder_id: uuid::Uuid,
    pub resource_type: String,
    pub access_level: String,
}
