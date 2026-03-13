use sqlx::{Pool, Postgres};
use crate::models::carbon_footprint::CarbonFootprint;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CarbonFootprintRepository {
    pool: Pool<Postgres>,
}

impl CarbonFootprintRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "carbon_footprint_insert", skip(self, model))]
    pub async fn insert(&self, model: &CarbonFootprint, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: carbon_footprint");
        debug!("Preparing insert query for carbon_footprint with columns: id, bpan, raw_material_emission, manufacturing_emission, transport_emission, usage_emission, recycling_emission, total_emission, verified, created_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for carbon_footprint insertion");

        let query_str = "INSERT INTO carbon_footprint (id, bpan, raw_material_emission, manufacturing_emission, transport_emission, usage_emission, recycling_emission, total_emission, verified, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.raw_material_emission)
            .bind(&model.manufacturing_emission)
            .bind(&model.transport_emission)
            .bind(&model.usage_emission)
            .bind(&model.recycling_emission)
            .bind(&model.total_emission)
            .bind(&model.verified)
            .bind(&model.created_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into carbon_footprint, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }
        
        // Now natively log this action into audit_logs
        let audit_query_str = "INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        
        trace!("Executing audit log insertion within the transaction...");
        let audit_result = sqlx::query(audit_query_str)
            .bind(Uuid::new_v4())
            .bind(actor_id)
            .bind("INSERT")
            .bind("carbon_footprint")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for carbon_footprint, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into carbon_footprint and logged transaction to audit_logs.");
        Ok(())
    }
}