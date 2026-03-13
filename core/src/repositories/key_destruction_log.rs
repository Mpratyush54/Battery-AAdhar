use sqlx::{Pool, Postgres};
use crate::models::key_destruction_log::KeyDestructionLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct KeyDestructionLogRepository {
    pool: Pool<Postgres>,
}

impl KeyDestructionLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "key_destruction_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &KeyDestructionLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: key_destruction_log");
        debug!("Preparing insert query for key_destruction_log with columns: id, bpan, dek_version, destroyed_by, destruction_method, destroyed_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for key_destruction_log insertion");

        let query_str = "INSERT INTO key_destruction_log (id, bpan, dek_version, destroyed_by, destruction_method, destroyed_at) VALUES ($1, $2, $3, $4, $5, $6)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.dek_version)
            .bind(&model.destroyed_by)
            .bind(&model.destruction_method)
            .bind(&model.destroyed_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into key_destruction_log, rolling back: {:?}", e);
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
            .bind("key_destruction_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for key_destruction_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into key_destruction_log and logged transaction to audit_logs.");
        Ok(())
    }
}