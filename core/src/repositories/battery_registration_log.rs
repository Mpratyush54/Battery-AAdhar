use sqlx::{Pool, Postgres};
use crate::models::battery_registration_log::BatteryRegistrationLog;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct BatteryRegistrationLogRepository {
    pool: Pool<Postgres>,
}

impl BatteryRegistrationLogRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "battery_registration_log_insert", skip(self, model))]
    pub async fn insert(&self, model: &BatteryRegistrationLog, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: battery_registration_log");
        debug!("Preparing insert query for battery_registration_log with columns: id, bpan, manufacturer_id, registration_status, submitted_at, approved_at, approved_by");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for battery_registration_log insertion");

        let query_str = "INSERT INTO battery_registration_log (id, bpan, manufacturer_id, registration_status, submitted_at, approved_at, approved_by) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.manufacturer_id)
            .bind(&model.registration_status)
            .bind(&model.submitted_at)
            .bind(&model.approved_at)
            .bind(&model.approved_by)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into battery_registration_log, rolling back: {:?}", e);
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
            .bind("battery_registration_log")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for battery_registration_log, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into battery_registration_log and logged transaction to audit_logs.");
        Ok(())
    }
}