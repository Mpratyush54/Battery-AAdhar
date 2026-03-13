use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DataClassification {
    pub id: uuid::Uuid,
    pub table_name: String,
    pub field_name: String,
    pub classification: String,
}
