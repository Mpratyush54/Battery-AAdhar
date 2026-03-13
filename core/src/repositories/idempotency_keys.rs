use sqlx::{Pool, Postgres};
use crate::models::idempotency_keys::IdempotencyKeys;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct IdempotencyKeysRepository {
    pool: Pool<Postgres>,
}

impl IdempotencyKeysRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "idempotency_keys_insert", skip(self, model))]
    pub async fn insert(&self, model: &IdempotencyKeys, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: idempotency_keys");
        debug!("Preparing insert query for idempotency_keys with columns: id, request_hash, response_hash, expires_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for idempotency_keys insertion");

        let query_str = "INSERT INTO idempotency_keys (id, request_hash, response_hash, expires_at) VALUES ($1, $2, $3, $4)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.request_hash)
            .bind(&model.response_hash)
            .bind(&model.expires_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into idempotency_keys, rolling back: {:?}", e);
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
            .bind("idempotency_keys")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for idempotency_keys, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into idempotency_keys and logged transaction to audit_logs.");
        Ok(())
    }
}