use sqlx::{Pool, Postgres};
use crate::models::batteries::Batteries;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct BatteriesRepository {
    pool: Pool<Postgres>,
}

impl BatteriesRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "batteries_insert", skip(self, model))]
    pub async fn insert(&self, model: &Batteries, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: batteries");
        debug!("Preparing insert query for batteries with columns: bpan, manufacturer_id, production_year, battery_category, compliance_class, static_hash, carbon_hash, created_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for batteries insertion");

        let query_str = "INSERT INTO batteries (bpan, manufacturer_id, production_year, battery_category, compliance_class, static_hash, carbon_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        let query = sqlx::query(query_str)
            .bind(&model.bpan)
            .bind(&model.manufacturer_id)
            .bind(&model.production_year)
            .bind(&model.battery_category)
            .bind(&model.compliance_class)
            .bind(&model.static_hash)
            .bind(&model.carbon_hash)
            .bind(&model.created_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into batteries, rolling back: {:?}", e);
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
            .bind("batteries")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for batteries, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into batteries and logged transaction to audit_logs.");
        Ok(())
    }
}