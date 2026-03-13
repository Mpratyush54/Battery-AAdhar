use sqlx::{Pool, Postgres};
use crate::models::qr_generation_log::QrGenerationLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct QrGenerationLogRepository {
    pool: Pool<Postgres>,
}

impl QrGenerationLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "qr_generation_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &QrGenerationLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: qr_generation_log");
        debug!("Preparing insert query for qr_generation_log with columns: id, bpan, qr_version, generated_by, generated_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for qr_generation_log insertion");

        let query_str = "INSERT INTO qr_generation_log (id, bpan, qr_version, generated_by, generated_at) VALUES ($1, $2, $3, $4, $5)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(1i32)
            .bind(&actor_id)
            .bind(&model.generated_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into qr_generation_log, rolling back: {:?}", e);
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
            .bind("qr_generation_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for qr_generation_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into qr_generation_log and logged transaction to audit_logs.");
        Ok(())
    }
}