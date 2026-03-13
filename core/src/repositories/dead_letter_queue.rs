use sqlx::{Pool, Postgres};
use crate::models::dead_letter_queue::DeadLetterQueue;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct DeadLetterQueueRepository {
    pool: Pool<Postgres>,
}

impl DeadLetterQueueRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "dead_letter_queue_insert", skip(self, model))]
    pub async fn insert(&self, model: &DeadLetterQueue, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: dead_letter_queue");
        debug!("Preparing insert query for dead_letter_queue with columns: id, original_message_id, failure_reason_hash, retry_count, created_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for dead_letter_queue insertion");

        let query_str = "INSERT INTO dead_letter_queue (id, original_message_id, failure_reason_hash, retry_count, created_at) VALUES ($1, $2, $3, $4, $5)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.original_message_id)
            .bind(&model.failure_reason_hash)
            .bind(&model.retry_count)
            .bind(&model.created_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into dead_letter_queue, rolling back: {:?}", e);
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
            .bind("dead_letter_queue")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for dead_letter_queue, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into dead_letter_queue and logged transaction to audit_logs.");
        Ok(())
    }
}