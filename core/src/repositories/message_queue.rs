use sqlx::{Pool, Postgres};
use crate::models::message_queue::MessageQueue;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct MessageQueueRepository {
    pool: Pool<Postgres>,
}

impl MessageQueueRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "message_queue_insert", skip(self, model))]
    pub async fn insert(&self, model: &MessageQueue, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: message_queue");
        debug!("Preparing insert query for message_queue with columns: id, cipher_algorithm, cipher_version, topic_hash, encrypted_payload, status_hash, created_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for message_queue insertion");

        let query_str = "INSERT INTO message_queue (id, cipher_algorithm, cipher_version, topic_hash, encrypted_payload, status_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.cipher_algorithm)
            .bind(&model.cipher_version)
            .bind(&model.topic_hash)
            .bind(&model.encrypted_payload)
            .bind(&model.status_hash)
            .bind(&model.created_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into message_queue, rolling back: {:?}", e);
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
            .bind("message_queue")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for message_queue, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into message_queue and logged transaction to audit_logs.");
        Ok(())
    }
}