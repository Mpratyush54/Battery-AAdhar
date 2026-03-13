use sqlx::{Pool, Postgres};
use crate::models::recycling_records::RecyclingRecords;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct RecyclingRecordsRepository {
    pool: Pool<Postgres>,
}

impl RecyclingRecordsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "recycling_records_insert", skip(self, model))]
    pub async fn insert(&self, model: &RecyclingRecords, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: recycling_records");
        debug!("Preparing insert query for recycling_records with columns: id, bpan, recycler_name, recovered_material_percentage, certificate_hash, recycled_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for recycling_records insertion");

        let query_str = "INSERT INTO recycling_records (id, bpan, recycler_name, recovered_material_percentage, certificate_hash, recycled_at) VALUES ($1, $2, $3, $4, $5, $6)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.recycler_name)
            .bind(&model.recovered_material_percentage)
            .bind(&model.certificate_hash)
            .bind(&model.recycled_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into recycling_records, rolling back: {:?}", e);
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
            .bind("recycling_records")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for recycling_records, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into recycling_records and logged transaction to audit_logs.");
        Ok(())
    }
}