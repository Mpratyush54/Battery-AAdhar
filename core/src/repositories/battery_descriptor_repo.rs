//! battery_descriptor_repo.rs — Battery descriptor persistence (immutable after creation)

use sqlx::{PgPool, Row};
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
        bpan: &str,
    ) -> Result<Option<BatteryDescriptor>, RepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT id, bpan, capacity_kwh, nominal_voltage_v, chemistry_type, cell_type, cell_count,
                   manufacturer_id, manufacturing_country, manufacturing_facility, manufacture_date,
                   declared_cycle_life, warranty_years, battery_hash, created_at
            FROM battery_descriptors
            WHERE bpan = $1
            LIMIT 1
            "#,
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(r) => {
                Ok(Some(BatteryDescriptor {
                    id: r.get("id"),
                    bpan: r.get("bpan"),
                    capacity_kwh: r.get("capacity_kwh"),
                    nominal_voltage_v: r.get("nominal_voltage_v"),
                    nominal_current_a: 0.0,
                    chemistry_type: r.get("chemistry_type"),
                    cell_type: r.get("cell_type"),
                    cell_count: r.get("cell_count"),
                    cell_voltage_nominal_v: 0.0,
                    manufacturer_id: r.get("manufacturer_id"),
                    manufacturing_country: r.get("manufacturing_country"),
                    manufacturing_facility: r.get("manufacturing_facility"),
                    manufacture_date: r.get("manufacture_date"),
                    declared_cycle_life: r.get("declared_cycle_life"),
                    warranty_years: r.get("warranty_years"),
                    registered_at: r.get("created_at"),
                    battery_hash: r.get("battery_hash"),
                }))
            }
            None => Ok(None),
        }
    }
}
