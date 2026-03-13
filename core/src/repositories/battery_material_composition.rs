use sqlx::{Pool, Postgres};
use crate::models::battery_material_composition::BatteryMaterialComposition;
use tracing::{info, error, debug, trace, instrument};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct BatteryMaterialCompositionRepository {
    pool: Pool<Postgres>,
}

impl BatteryMaterialCompositionRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    #[instrument(name = "battery_material_composition_insert", skip(self, model))]
    pub async fn insert(&self, model: &BatteryMaterialComposition, actor_id: Uuid) -> Result<(), sqlx::Error> {
        trace!("Entering transactional insert function for table: battery_material_composition");
        debug!("Preparing insert query for battery_material_composition with columns: id, bpan, cathode_material, anode_material, electrolyte_type, separator_material, lithium_content_g, cobalt_content_g, nickel_content_g, recyclable_percentage, encrypted_details, created_at");

        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        trace!("Transaction started for battery_material_composition insertion");

        let query_str = "INSERT INTO battery_material_composition (id, bpan, cathode_material, anode_material, electrolyte_type, separator_material, lithium_content_g, cobalt_content_g, nickel_content_g, recyclable_percentage, encrypted_details, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)";
        let query = sqlx::query(query_str)
            .bind(&model.id)
            .bind(&model.bpan)
            .bind(&model.cathode_material)
            .bind(&model.anode_material)
            .bind(&model.electrolyte_type)
            .bind(&model.separator_material)
            .bind(&model.lithium_content_g)
            .bind(&model.cobalt_content_g)
            .bind(&model.nickel_content_g)
            .bind(&model.recyclable_percentage)
            .bind(&model.encrypted_details)
            .bind(&model.created_at)
            ;

        trace!("Executing primary query against the transaction...");
        let primary_result = query.execute(&mut *tx).await;
        if let Err(e) = primary_result {
            error!("Failed to insert into battery_material_composition, rolling back: {:?}", e);
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
            .bind("battery_material_composition")
            .bind("NONE") // previous_hash
            .bind("TODO_HASH") // entry_hash computation usually goes here
            .bind(Utc::now().naive_utc())
            .execute(&mut *tx).await;
            
        if let Err(e) = audit_result {
            error!("Failed to insert into audit_logs for battery_material_composition, rolling back: {:?}", e);
            let _ = tx.rollback().await;
            return Err(e);
        }

        trace!("Committing transaction...");
        tx.commit().await?;
        
        info!("Successfully inserted record into battery_material_composition and logged transaction to audit_logs.");
        Ok(())
    }
}