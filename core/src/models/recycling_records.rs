use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecyclingRecords {
    pub id: uuid::Uuid,
    pub bpan: String,
    pub recycler_name: String,
    pub recovered_material_percentage: f64,
    pub certificate_hash: String,
    pub recycled_at: chrono::NaiveDateTime,
}
