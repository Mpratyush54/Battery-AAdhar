use sqlx::{Pool, Postgres};
use crate::models::battery_keys::BatteryKeys;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct BatteryKeysRepository {
    pool: Pool<Postgres>,
}

impl BatteryKeysRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "battery_keys_insert", skip(self, model))]
    pub async fn insert(&self, model: &BatteryKeys, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: battery_keys");
        debug!("Preparing insert query for battery_keys with columns: bpan, encrypted_dek, kek_version, cipher_algorithm, cipher_version, key_status, created_at, rotated_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for battery_keys insertion");

        let query_str = "INSERT INTO battery_keys (bpan, encrypted_dek, kek_version, cipher_algorithm, cipher_version, key_status, created_at, rotated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
        let query = sqlx::query(query_str)
            .bind(&model.bpan)
            .bind(&model.encrypted_dek)
            .bind(&model.kek_version)
            .bind(&model.cipher_algorithm)
            .bind(&model.cipher_version)
            .bind(&model.key_status)
            .bind(&model.created_at)
            .bind(&model.rotated_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into battery_keys, rolling back: {:?}", e);
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
            .bind("battery_keys")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for battery_keys, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into battery_keys and logged transaction to audit_logs.");
        Ok(())
    }
}