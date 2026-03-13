use sqlx::{Pool, Postgres};
use crate::models::rate_limits::RateLimits;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct RateLimitsRepository {
    pool: Pool<Postgres>,
}

impl RateLimitsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "rate_limits_insert", skip(self, model))]
    pub async fn insert(&self, model: &RateLimits, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: rate_limits");
        debug!("Preparing insert query for rate_limits with columns: id, subject_hash, window_start, request_count");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for rate_limits insertion");

        let query_str = "INSERT INTO rate_limits (id, subject_hash, window_start, request_count) VALUES ($1, $2, $3, $4)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.subject_hash)
            .bind(&model.window_start)
            .bind(&model.request_count)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into rate_limits, rolling back: {:?}", e);
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
            .bind("rate_limits")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for rate_limits, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into rate_limits and logged transaction to audit_logs.");
        Ok(())
    }
}