use sqlx::{Pool, Postgres};
use crate::models::scheduled_jobs::ScheduledJobs;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ScheduledJobsRepository {
    pool: Pool<Postgres>,
}

impl ScheduledJobsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "scheduled_jobs_insert", skip(self, model))]
    pub async fn insert(&self, model: &ScheduledJobs, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: scheduled_jobs");
        debug!("Preparing insert query for scheduled_jobs with columns: id, job_name_hash, cron_expression, enabled, last_run");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for scheduled_jobs insertion");

        let query_str = "INSERT INTO scheduled_jobs (id, job_name_hash, cron_expression, enabled, last_run) VALUES ($1, $2, $3, $4, $5)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.job_name_hash)
            .bind(&model.cron_expression)
            .bind(&model.enabled)
            .bind(&model.last_run)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into scheduled_jobs, rolling back: {:?}", e);
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
            .bind("scheduled_jobs")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for scheduled_jobs, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into scheduled_jobs and logged transaction to audit_logs.");
        Ok(())
    }
}