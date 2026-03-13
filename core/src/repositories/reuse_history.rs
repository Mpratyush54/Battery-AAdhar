use sqlx::{Pool, Postgres};
use crate::models::reuse_history::ReuseHistory;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ReuseHistoryRepository {
    pool: Pool<Postgres>,
}

impl ReuseHistoryRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "reuse_history_insert", skip(self, model))]
    pub async fn insert(&self, model: &ReuseHistory, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: reuse_history");
        debug!("Preparing insert query for reuse_history with columns: id, bpan, reuse_application, certified_by, certified_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for reuse_history insertion");

        let query_str = "INSERT INTO reuse_history (id, bpan, reuse_application, certified_by, certified_at) VALUES ($1, $2, $3, $4, $5)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.reuse_application)
            .bind(&model.certified_by)
            .bind(&model.certified_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into reuse_history, rolling back: {:?}", e);
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
            .bind("reuse_history")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for reuse_history, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into reuse_history and logged transaction to audit_logs.");
        Ok(())
    }
}