use sqlx::{Pool, Postgres};
use crate::models::key_rotation_log::KeyRotationLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct KeyRotationLogRepository {
    pool: Pool<Postgres>,
}

impl KeyRotationLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "key_rotation_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &KeyRotationLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: key_rotation_log");
        debug!("Preparing insert query for key_rotation_log with columns: id, key_type, previous_version, new_version, initiated_by, approved_by, approval_timestamp, rotated_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for key_rotation_log insertion");

        let query_str = "INSERT INTO key_rotation_log (id, key_type, previous_version, new_version, initiated_by, approved_by, approval_timestamp, rotated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.key_type)
            .bind(&model.previous_version)
            .bind(&model.new_version)
            .bind(&model.initiated_by)
            .bind(&model.approved_by)
            .bind(&model.approval_timestamp)
            .bind(&model.rotated_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into key_rotation_log, rolling back: {:?}", e);
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
            .bind("key_rotation_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for key_rotation_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into key_rotation_log and logged transaction to audit_logs.");
        Ok(())
    }
}