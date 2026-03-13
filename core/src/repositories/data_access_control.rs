use sqlx::{Pool, Postgres};
use crate::models::data_access_control::DataAccessControl;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct DataAccessControlRepository {
    pool: Pool<Postgres>,
}

impl DataAccessControlRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "data_access_control_insert", skip(self, model))]
    pub async fn insert(&self, model: &DataAccessControl, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: data_access_control");
        debug!("Preparing insert query for data_access_control with columns: id, stakeholder_id, resource_type, access_level");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for data_access_control insertion");

        let query_str = "INSERT INTO data_access_control (id, stakeholder_id, resource_type, access_level) VALUES ($1, $2, $3, $4)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.stakeholder_id)
            .bind(&model.resource_type)
            .bind(&model.access_level)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into data_access_control, rolling back: {:?}", e);
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
            .bind("data_access_control")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for data_access_control, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into data_access_control and logged transaction to audit_logs.");
        Ok(())
    }
}