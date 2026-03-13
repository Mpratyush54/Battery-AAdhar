use sqlx::{Pool, Postgres};
use crate::models::system_metrics::SystemMetrics;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SystemMetricsRepository {
    pool: Pool<Postgres>,
}

impl SystemMetricsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "system_metrics_insert", skip(self, model))]
    pub async fn insert(&self, model: &SystemMetrics, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: system_metrics");
        debug!("Preparing insert query for system_metrics with columns: id, cipher_algorithm, cipher_version, metric_name_hash, metric_value_cipher, recorded_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for system_metrics insertion");

        let query_str = "INSERT INTO system_metrics (id, cipher_algorithm, cipher_version, metric_name_hash, metric_value_cipher, recorded_at) VALUES ($1, $2, $3, $4, $5, $6)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.cipher_algorithm)
            .bind(&model.cipher_version)
            .bind(&model.metric_name_hash)
            .bind(&model.metric_value_cipher)
            .bind(&model.recorded_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into system_metrics, rolling back: {:?}", e);
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
            .bind("system_metrics")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for system_metrics, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into system_metrics and logged transaction to audit_logs.");
        Ok(())
    }
}