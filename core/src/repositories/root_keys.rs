use sqlx::{Pool, Postgres};
use crate::models::root_keys::RootKeys;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct RootKeysRepository {
    pool: Pool<Postgres>,
}

impl RootKeysRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "root_keys_insert", skip(self, model))]
    pub async fn insert(&self, model: &RootKeys, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: root_keys");
        debug!("Preparing insert query for root_keys with columns: id, key_identifier, hardware_backed, status, created_at, retired_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for root_keys insertion");

        let query_str = "INSERT INTO root_keys (id, key_identifier, hardware_backed, status, created_at, retired_at) VALUES ($1, $2, $3, $4, $5, $6)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.key_identifier)
            .bind(&model.hardware_backed)
            .bind(&model.status)
            .bind(&model.created_at)
            .bind(&model.retired_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into root_keys, rolling back: {:?}", e);
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
            .bind("root_keys")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for root_keys, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into root_keys and logged transaction to audit_logs.");
        Ok(())
    }
}