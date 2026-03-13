use sqlx::{Pool, Postgres};
use crate::models::battery_identifiers::BatteryIdentifiers;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct BatteryIdentifiersRepository {
    pool: Pool<Postgres>,
}

impl BatteryIdentifiersRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "battery_identifiers_insert", skip(self, model))]
    pub async fn insert(&self, model: &BatteryIdentifiers, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: battery_identifiers");
        debug!("Preparing insert query for battery_identifiers with columns: id, bpan, cipher_algorithm, cipher_version, encrypted_serial_number, encrypted_batch_number, encrypted_factory_code, created_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for battery_identifiers insertion");

        let query_str = "INSERT INTO battery_identifiers (id, bpan, cipher_algorithm, cipher_version, encrypted_serial_number, encrypted_batch_number, encrypted_factory_code, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.cipher_algorithm)
            .bind(&model.cipher_version)
            .bind(&model.encrypted_serial_number)
            .bind(&model.encrypted_batch_number)
            .bind(&model.encrypted_factory_code)
            .bind(&model.created_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into battery_identifiers, rolling back: {:?}", e);
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
            .bind("battery_identifiers")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for battery_identifiers, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into battery_identifiers and logged transaction to audit_logs.");
        Ok(())
    }
}