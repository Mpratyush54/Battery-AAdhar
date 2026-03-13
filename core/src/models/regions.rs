use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Regions {
    pub id: uuid::Uuid,
    pub region_hash: String,
    pub data_center_hash: String,
}
