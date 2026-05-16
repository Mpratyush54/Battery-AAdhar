//! carbon_repo.rs — Carbon footprint persistence with verification
//!
//! Stores BCF data + verification status + audit trail.
//! Table: carbon_footprint (see dbschma.txt line 100)

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::CarbonFootprint;

#[derive(Debug)]
pub enum RepositoryError {
    DatabaseError(String),
    NotFound(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::DatabaseError(msg) => write!(f, "db error: {}", msg),
            RepositoryError::NotFound(msg) => write!(f, "not found: {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

#[async_trait]
pub trait CarbonRepository: Send + Sync {
    async fn insert_carbon_footprint(&self, cf: &CarbonFootprint) -> Result<String, RepositoryError>;
    async fn get_by_bpan(&self, bpan: &str) -> Result<Option<CarbonFootprint>, RepositoryError>;
    async fn verify_carbon_footprint(
        &self,
        bpan: &str,
        verified_by: &str,
        standard: &str,
    ) -> Result<(), RepositoryError>;
}

pub struct CarbonRepositoryImpl {
    pool: PgPool,
}

impl CarbonRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        CarbonRepositoryImpl { pool }
    }
}

#[async_trait]
impl CarbonRepository for CarbonRepositoryImpl {
    async fn insert_carbon_footprint(&self, cf: &CarbonFootprint) -> Result<String, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let carbon_hash = cf.recompute_hash();

        sqlx::query(
            r#"
            INSERT INTO carbon_footprint 
            (id, bpan, raw_material_emission, manufacturing_emission, transport_emission,
             usage_emission, recycling_emission, total_emission, verified, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(id)
        .bind(&cf.bpan)
        .bind(cf.raw_material_emissions_kg_co2e as f64)
        .bind(cf.manufacturing_emissions_kg_co2e as f64)
        .bind(cf.transport_emissions_kg_co2e as f64)
        .bind(cf.usage_emissions_kg_co2e as f64)
        .bind(cf.recycling_emissions_kg_co2e as f64)
        .bind(cf.total_emissions_kg_co2e as f64)
        .bind(cf.verified)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // Log to submission audit trail
        sqlx::query(
            r#"
            INSERT INTO static_data_submission_log 
            (id, bpan, submitted_by, data_type, version, submitted_at)
            VALUES ($1, $2, $3, 'BCF', $4, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&cf.bpan)
        .bind(&cf.submitted_by)
        .bind(cf.submitted_version)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id.to_string())
    }

    async fn get_by_bpan(&self, bpan: &str) -> Result<Option<CarbonFootprint>, RepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT bpan, raw_material_emission, manufacturing_emission, transport_emission,
                   usage_emission, recycling_emission, total_emission, verified, created_at
            FROM carbon_footprint WHERE bpan = $1
            "#,
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(r) => {
                use sqlx::Row;
                let created_at: chrono::NaiveDateTime = r.get("created_at");
                let cf = CarbonFootprint {
                    bpan: r.get("bpan"),
                    raw_material_emissions_kg_co2e: r.get::<f64, _>("raw_material_emission") as f32,
                    raw_material_source_country: String::new(),
                    mining_method: String::new(),
                    manufacturing_emissions_kg_co2e: r.get::<f64, _>("manufacturing_emission") as f32,
                    manufacturing_location: String::new(),
                    factory_energy_source: String::new(),
                    cell_production_method: String::new(),
                    transport_emissions_kg_co2e: r.get::<f64, _>("transport_emission") as f32,
                    transport_distance_km: 0.0,
                    transport_mode: String::new(),
                    transport_packaging: String::new(),
                    usage_emissions_kg_co2e: r.get::<f64, _>("usage_emission") as f32,
                    usage_years: 0,
                    usage_grid_emissions_factor: 0.0,
                    usage_annual_km: 0,
                    recycling_emissions_kg_co2e: r.get::<f64, _>("recycling_emission") as f32,
                    recycling_recovery_rate: 0.0,
                    recycling_avoided_mining: 0.0,
                    recycling_method: String::new(),
                    total_emissions_kg_co2e: r.get::<f64, _>("total_emission") as f32,
                    emissions_per_kwh: 0.0,
                    carbon_hash: String::new(),
                    submitted_by: String::new(),
                    submitted_at: created_at.and_utc(),
                    submitted_version: 1,
                    verified: r.get("verified"),
                    verified_by: None,
                    verified_at: None,
                    verification_standard: None,
                };
                Ok(Some(cf))
            }
            None => Ok(None),
        }
    }

    async fn verify_carbon_footprint(
        &self,
        bpan: &str,
        verified_by: &str,
        _standard: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE carbon_footprint 
            SET verified = true
            WHERE bpan = $1
            "#,
        )
        .bind(bpan)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // Log verification in submission log
        sqlx::query(
            r#"
            INSERT INTO static_data_submission_log 
            (id, bpan, submitted_by, data_type, version, submitted_at)
            VALUES ($1, $2, $3, 'BCF_VERIFY', 1, $4)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(bpan)
        .bind(verified_by)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require real Postgres — see core/tests/
}
