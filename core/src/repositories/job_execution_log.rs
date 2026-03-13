use sqlx::{Pool, Postgres};
use crate::models::job_execution_log::JobExecutionLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct JobExecutionLogRepository {
    pool: Pool<Postgres>,
}

impl JobExecutionLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "job_execution_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &JobExecutionLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: job_execution_log");
        debug!("Preparing insert query for job_execution_log with columns: id, job_id, status_hash, started_at, finished_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for job_execution_log insertion");

        let query_str = "INSERT INTO job_execution_log (id, job_id, status_hash, started_at, finished_at) VALUES ($1, $2, $3, $4, $5)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.job_id)
            .bind(&model.status)
            .bind(&model.duration_ms)
            .bind(&model.executed_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into job_execution_log, rolling back: {:?}", e);
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
            .bind("job_execution_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for job_execution_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into job_execution_log and logged transaction to audit_logs.");
        Ok(())
    }
}