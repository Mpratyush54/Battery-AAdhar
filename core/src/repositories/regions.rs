use sqlx::{Pool, Postgres};
use crate::models::regions::Regions;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct RegionsRepository {
    pool: Pool<Postgres>,
}

impl RegionsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "regions_insert", skip(self, model))]
    pub async fn insert(&self, model: &Regions, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: regions");
        debug!("Preparing insert query for regions with columns: id, region_hash, data_center_hash");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for regions insertion");

        let query_str = "INSERT INTO regions (id, region_hash, data_center_hash) VALUES ($1, $2, $3)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.region_hash)
            .bind(&model.data_center_hash)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into regions, rolling back: {:?}", e);
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
            .bind("regions")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for regions, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into regions and logged transaction to audit_logs.");
        Ok(())
    }
}