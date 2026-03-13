use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationLog {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub validation_type: String,
    pub validation_result: String,
    pub remarks: String,
    pub validated_by: uuid::Uuid,
    pub validated_at: chrono::NaiveDateTime,
}
