use sqlx::{Pool, Postgres};
use crate::models::dynamic_data_log::DynamicDataLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct DynamicDataLogRepository {
    pool: Pool<Postgres>,
}

impl DynamicDataLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "dynamic_data_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &DynamicDataLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: dynamic_data_log");
        debug!("Preparing insert query for dynamic_data_log with columns: id, bpan, previous_event_hash, event_hash, upload_type, record_hash, uploaded_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for dynamic_data_log insertion");

        let query_str = "INSERT INTO dynamic_data_log (id, bpan, previous_event_hash, event_hash, upload_type, record_hash, uploaded_at) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.previous_event_hash)
            .bind(&model.event_hash)
            .bind(&model.upload_type)
            .bind(&model.record_hash)
            .bind(&model.uploaded_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into dynamic_data_log, rolling back: {:?}", e);
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
            .bind("dynamic_data_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for dynamic_data_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into dynamic_data_log and logged transaction to audit_logs.");
        Ok(())
    }
}