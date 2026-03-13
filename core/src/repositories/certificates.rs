use sqlx::{Pool, Postgres};
use crate::models::certificates::Certificates;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CertificatesRepository {
    pool: Pool<Postgres>,
}

impl CertificatesRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "certificates_insert", skip(self, model))]
    pub async fn insert(&self, model: &Certificates, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: certificates");
        debug!("Preparing insert query for certificates with columns: id, public_key, issued_by_hash, issued_at, expires_at, revoked");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for certificates insertion");

        let query_str = "INSERT INTO certificates (id, public_key, issued_by_hash, issued_at, expires_at, revoked) VALUES ($1, $2, $3, $4, $5, $6)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.public_key)
            .bind(&model.issued_by_hash)
            .bind(&model.issued_at)
            .bind(&model.expires_at)
            .bind(&model.revoked)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into certificates, rolling back: {:?}", e);
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
            .bind("certificates")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for certificates, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into certificates and logged transaction to audit_logs.");
        Ok(())
    }
}