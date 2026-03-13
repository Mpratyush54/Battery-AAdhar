use sqlx::{Pool, Postgres};
use crate::models::api_requests::ApiRequests;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiRequestsRepository {
    pool: Pool<Postgres>,
}

impl ApiRequestsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "api_requests_insert", skip(self, model))]
    pub async fn insert(&self, model: &ApiRequests, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: api_requests");
        debug!("Preparing insert query for api_requests with columns: id, parent_request_id, request_hash, endpoint_hash, subject_hash, status_hash, latency_ms, created_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for api_requests insertion");

        let query_str = "INSERT INTO api_requests (id, parent_request_id, request_hash, endpoint_hash, subject_hash, status_hash, latency_ms, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.parent_request_id)
            .bind(&model.request_hash)
            .bind(&model.endpoint_hash)
            .bind(&model.subject_hash)
            .bind(&model.status_hash)
            .bind(&model.latency_ms)
            .bind(&model.created_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into api_requests, rolling back: {:?}", e);
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
            .bind("api_requests")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for api_requests, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into api_requests and logged transaction to audit_logs.");
        Ok(())
    }
}