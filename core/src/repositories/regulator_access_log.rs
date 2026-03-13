use sqlx::{Pool, Postgres};
use crate::models::regulator_access_log::RegulatorAccessLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct RegulatorAccessLogRepository {
    pool: Pool<Postgres>,
}

impl RegulatorAccessLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "regulator_access_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &RegulatorAccessLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: regulator_access_log");
        debug!("Preparing insert query for regulator_access_log with columns: id, stakeholder_id, bpan, reason, accessed_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for regulator_access_log insertion");

        let query_str = "INSERT INTO regulator_access_log (id, stakeholder_id, bpan, reason, accessed_at) VALUES ($1, $2, $3, $4, $5)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.stakeholder_id)
            .bind(&model.bpan)
            .bind(&model.reason)
            .bind(&model.accessed_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into regulator_access_log, rolling back: {:?}", e);
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
            .bind("regulator_access_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for regulator_access_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into regulator_access_log and logged transaction to audit_logs.");
        Ok(())
    }
}