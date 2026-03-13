use sqlx::{Pool, Postgres};
use crate::models::static_data_update_log::StaticDataUpdateLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct StaticDataUpdateLogRepository {
    pool: Pool<Postgres>,
}

impl StaticDataUpdateLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "static_data_update_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &StaticDataUpdateLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: static_data_update_log");
        debug!("Preparing insert query for static_data_update_log with columns: id, bpan, field_name, old_hash, new_hash, updated_by, updated_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for static_data_update_log insertion");

        let query_str = "INSERT INTO static_data_update_log (id, bpan, field_name, old_hash, new_hash, updated_by, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.field_name)
            .bind(&model.previous_hash)
            .bind(&model.new_hash)
            .bind(&model.updated_by)
            .bind(&model.updated_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into static_data_update_log, rolling back: {:?}", e);
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
            .bind("static_data_update_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for static_data_update_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into static_data_update_log and logged transaction to audit_logs.");
        Ok(())
    }
}