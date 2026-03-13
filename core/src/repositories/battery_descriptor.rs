use sqlx::{Pool, Postgres};
use crate::models::battery_descriptor::BatteryDescriptor;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct BatteryDescriptorRepository {
    pool: Pool<Postgres>,
}

impl BatteryDescriptorRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "battery_descriptor_insert", skip(self, model))]
    pub async fn insert(&self, model: &BatteryDescriptor, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: battery_descriptor");
        debug!("Preparing insert query for battery_descriptor with columns: id, bpan, chemistry_type, nominal_voltage, rated_capacity_kwh, energy_density, weight_kg, form_factor, created_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for battery_descriptor insertion");

        let query_str = "INSERT INTO battery_descriptor (id, bpan, chemistry_type, nominal_voltage, rated_capacity_kwh, energy_density, weight_kg, form_factor, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.chemistry_type)
            .bind(&model.nominal_voltage)
            .bind(&model.rated_capacity_kwh)
            .bind(&model.energy_density)
            .bind(&model.weight_kg)
            .bind(&model.form_factor)
            .bind(&model.created_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into battery_descriptor, rolling back: {:?}", e);
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
            .bind("battery_descriptor")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for battery_descriptor, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into battery_descriptor and logged transaction to audit_logs.");
        Ok(())
    }
}