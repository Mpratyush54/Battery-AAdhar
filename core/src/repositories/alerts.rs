use sqlx::{Pool, Postgres};
use crate::models::alerts::Alerts;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AlertsRepository {
    pool: Pool<Postgres>,
}

impl AlertsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "alerts_insert", skip(self, model))]
    pub async fn insert(&self, model: &Alerts, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: alerts");
        debug!("Preparing insert query for alerts with columns: id, cipher_algorithm, cipher_version, severity_hash, message_cipher, triggered_at, resolved");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for alerts insertion");

        let query_str = "INSERT INTO alerts (id, cipher_algorithm, cipher_version, severity_hash, message_cipher, triggered_at, resolved) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.cipher_algorithm)
            .bind(&model.cipher_version)
            .bind(&model.severity_hash)
            .bind(&model.message_cipher)
            .bind(&model.triggered_at)
            .bind(&model.resolved)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into alerts, rolling back: {:?}", e);
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
            .bind("alerts")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for alerts, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into alerts and logged transaction to audit_logs.");
        Ok(())
    }
}