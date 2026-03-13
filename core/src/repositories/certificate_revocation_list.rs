use sqlx::{Pool, Postgres};
use crate::models::certificate_revocation_list::CertificateRevocationList;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CertificateRevocationListRepository {
    pool: Pool<Postgres>,
}

impl CertificateRevocationListRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "certificate_revocation_list_insert", skip(self, model))]
    pub async fn insert(&self, model: &CertificateRevocationList, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: certificate_revocation_list");
        debug!("Preparing insert query for certificate_revocation_list with columns: id, certificate_id, revoked_at, reason_hash");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for certificate_revocation_list insertion");

        let query_str = "INSERT INTO certificate_revocation_list (id, certificate_id, revoked_at, reason_hash) VALUES ($1, $2, $3, $4)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.certificate_id)
            .bind(&model.revoked_at)
            .bind(&model.revoked_by_hash)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into certificate_revocation_list, rolling back: {:?}", e);
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
            .bind("certificate_revocation_list")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for certificate_revocation_list, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into certificate_revocation_list and logged transaction to audit_logs.");
        Ok(())
    }
}