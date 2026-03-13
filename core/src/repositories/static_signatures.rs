use sqlx::{Pool, Postgres};
use crate::models::static_signatures::StaticSignatures;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct StaticSignaturesRepository {
    pool: Pool<Postgres>,
}

impl StaticSignaturesRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "static_signatures_insert", skip(self, model))]
    pub async fn insert(&self, model: &StaticSignatures, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: static_signatures");
        debug!("Preparing insert query for static_signatures with columns: id, bpan, data_section, data_hash, signature, certificate_id, signed_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for static_signatures insertion");

        let query_str = "INSERT INTO static_signatures (id, bpan, data_section, data_hash, signature, certificate_id, signed_at) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.data_section)
            .bind(&model.data_hash)
            .bind(&model.signature)
            .bind(&model.certificate_id)
            .bind(&model.signed_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into static_signatures, rolling back: {:?}", e);
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
            .bind("static_signatures")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for static_signatures, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into static_signatures and logged transaction to audit_logs.");
        Ok(())
    }
}