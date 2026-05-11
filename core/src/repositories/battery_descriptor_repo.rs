//! battery_descriptor_repo.rs — Battery descriptor persistence (immutable after creation)

use sqlx::PgPool;
use chrono::Utc;
use crate::models::BatteryDescriptor;

#[derive(Debug)]
pub enum RepositoryError {
    DatabaseError(String),
}

pub struct BatteryDescriptorRepositoryImpl {
    pool: PgPool,
}

impl BatteryDescriptorRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        BatteryDescriptorRepositoryImpl { pool }
    }

    /// Store battery descriptor (immutable)
    pub async fn insert(
        &self,
        descriptor: &BatteryDescriptor,
    ) -> Result<String, RepositoryError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO battery_descriptors
            (bpan, capacity_kwh, nominal_voltage_v, chemistry_type, cell_type, cell_count,
             manufacturer_id, manufacturing_country, manufacturing_facility, manufacture_date,
             declared_cycle_life, warranty_years, battery_hash, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(&descriptor.bpan)
        .bind(descriptor.capacity_kwh)
        .bind(descriptor.nominal_voltage_v)
        .bind(&descriptor.chemistry_type)
        .bind(&descriptor.cell_type)
        .bind(descriptor.cell_count as i32)
        .bind(descriptor.manufacturer_id) // Changed from string to UUID
        .bind(&descriptor.manufacturing_country)
        .bind(&descriptor.manufacturing_facility)
        .bind(&descriptor.manufacture_date)
        .bind(descriptor.declared_cycle_life as i32)
        .bind(descriptor.warranty_years as i32)
        .bind(&descriptor.battery_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(descriptor.bpan.clone())
    }

    /// Retrieve descriptor
    pub async fn get(
        &self,
        _bpan: &str,
    ) -> Result<Option<BatteryDescriptor>, RepositoryError> {
        // TODO: Fetch and reconstruct BatteryDescriptor
        Ok(None)
    }
}
