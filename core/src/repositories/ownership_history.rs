use sqlx::{Pool, Postgres};
use crate::models::ownership_history::OwnershipHistory;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct OwnershipHistoryRepository {
    pool: Pool<Postgres>,
}

impl OwnershipHistoryRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "ownership_history_insert", skip(self, model))]
    pub async fn insert(&self, model: &OwnershipHistory, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: ownership_history");
        debug!("Preparing insert query for ownership_history with columns: id, bpan, cipher_algorithm, cipher_version, encrypted_owner_identity, start_time, end_time");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for ownership_history insertion");

        let query_str = "INSERT INTO ownership_history (id, bpan, cipher_algorithm, cipher_version, encrypted_owner_identity, start_time, end_time) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.cipher_algorithm)
            .bind(&model.cipher_version)
            .bind(&model.encrypted_owner_identity)
            .bind(&model.start_time)
            .bind(&model.end_time)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into ownership_history, rolling back: {:?}", e);
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
            .bind("ownership_history")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for ownership_history, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into ownership_history and logged transaction to audit_logs.");
        Ok(())
    }
}