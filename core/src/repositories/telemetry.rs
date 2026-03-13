use sqlx::{Pool, Postgres};
use crate::models::telemetry::Telemetry;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct TelemetryRepository {
    pool: Pool<Postgres>,
}

impl TelemetryRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "telemetry_insert", skip(self, model))]
    pub async fn insert(&self, model: &Telemetry, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: telemetry");
        debug!("Preparing insert query for telemetry with columns: id, bpan, cipher_algorithm, cipher_version, encrypted_payload, recorded_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for telemetry insertion");

        let query_str = "INSERT INTO telemetry (id, bpan, cipher_algorithm, cipher_version, encrypted_payload, recorded_at) VALUES ($1, $2, $3, $4, $5, $6)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.cipher_algorithm)
            .bind(&model.cipher_version)
            .bind(&model.encrypted_payload)
            .bind(&model.recorded_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into telemetry, rolling back: {:?}", e);
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
            .bind("telemetry")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for telemetry, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into telemetry and logged transaction to audit_logs.");
        Ok(())
    }
}