use sqlx::{Pool, Postgres};
use crate::models::static_data_submission_log::StaticDataSubmissionLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct StaticDataSubmissionLogRepository {
    pool: Pool<Postgres>,
}

impl StaticDataSubmissionLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "static_data_submission_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &StaticDataSubmissionLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: static_data_submission_log");
        debug!("Preparing insert query for static_data_submission_log with columns: id, bpan, data_type, version, submission_status, submitted_at, reviewed_at, reviewer_id");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        trace!("Transaction started for static_data_submission_log insertion");

        let query_str = "INSERT INTO static_data_submission_log (id, bpan, data_type, version, submission_status, submitted_at, reviewed_at, reviewer_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.data_section)
            .bind(1i32)
            .bind(&model.data_hash)
            .bind(&model.submitted_at)
            .bind(&now)
            .bind(&actor_id)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into static_data_submission_log, rolling back: {:?}", e);
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
            .bind("static_data_submission_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for static_data_submission_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into static_data_submission_log and logged transaction to audit_logs.");
        Ok(())
    }
}