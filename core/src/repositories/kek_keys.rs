use sqlx::{Pool, Postgres};
use crate::models::kek_keys::KekKeys;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct KekKeysRepository {
    pool: Pool<Postgres>,
}

impl KekKeysRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "kek_keys_insert", skip(self, model))]
    pub async fn insert(&self, model: &KekKeys, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: kek_keys");
        debug!("Preparing insert query for kek_keys with columns: id, encrypted_kek, version, root_key_id, cipher_algorithm, cipher_version, status, created_at, retired_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for kek_keys insertion");

        let query_str = "INSERT INTO kek_keys (id, encrypted_kek, version, root_key_id, cipher_algorithm, cipher_version, status, created_at, retired_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.encrypted_kek)
            .bind(&model.version)
            .bind(&model.root_key_id)
            .bind(&model.cipher_algorithm)
            .bind(&model.cipher_version)
            .bind(&model.status)
            .bind(&model.created_at)
            .bind(&model.retired_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into kek_keys, rolling back: {:?}", e);
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
            .bind("kek_keys")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for kek_keys, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into kek_keys and logged transaction to audit_logs.");
        Ok(())
    }
}