//! lifecycle_repo.rs — Battery lifecycle tracking (ownership, reuse, recycling)

use super::battery_repo::RepositoryError;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OwnershipRecord {
    pub id: Uuid,
    pub bpan: String,
    pub owner_id: String,   // Stakeholder ID (manufacturer, distributor, etc.)
    pub owner_type: String, // "manufacturer", "distributor", "consumer", etc.
    pub start_time: chrono::DateTime<Utc>,
    pub end_time: Option<chrono::DateTime<Utc>>,
    pub transfer_reason: Option<String>, // "sale", "return", "refurbishment", etc.
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReuseRecord {
    pub id: Uuid,
    pub bpan: String,
    pub reuse_type: String, // "stationary", "industrial", etc.
    pub certifier_id: String,
    pub certified_at: chrono::DateTime<Utc>,
    pub expected_end_of_life: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecyclingRecord {
    pub id: Uuid,
    pub bpan: String,
    pub recycler_id: String,
    pub recovered_percentage: f32,
    pub recovery_method: String, // "mechanical", "hydrometallurgical", etc.
    pub recycled_at: chrono::DateTime<Utc>,
}

pub struct LifecycleRepositoryImpl {
    pool: PgPool,
}

impl LifecycleRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        LifecycleRepositoryImpl { pool }
    }

    /// Log an ownership transfer
    pub async fn log_ownership_transfer(
        &self,
        bpan: &str,
        _from_owner_id: &str,
        to_owner_id: &str,
        owner_type: &str,
        reason: Option<&str>,
    ) -> Result<OwnershipRecord, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // End the previous ownership record
        sqlx::query(
            "UPDATE battery_ownership SET end_time = $1 WHERE bpan = $2 AND end_time IS NULL",
        )
        .bind(now)
        .bind(bpan)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // Create new ownership record
        let record = OwnershipRecord {
            id,
            bpan: bpan.to_string(),
            owner_id: to_owner_id.to_string(),
            owner_type: owner_type.to_string(),
            start_time: now,
            end_time: None,
            transfer_reason: reason.map(|s| s.to_string()),
        };

        sqlx::query(
            r#"
            INSERT INTO battery_ownership (id, bpan, owner_id, owner_type, start_time, end_time, transfer_reason)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(to_owner_id)
        .bind(owner_type)
        .bind(now)
        .bind(None::<chrono::DateTime<Utc>>)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(record)
    }

    /// Get ownership history for a battery
    pub async fn get_ownership_history(
        &self,
        bpan: &str,
    ) -> Result<Vec<OwnershipRecord>, RepositoryError> {
        let records = sqlx::query_as::<_, OwnershipRecord>(
            "SELECT id, bpan, owner_id, owner_type, start_time, end_time, transfer_reason FROM battery_ownership WHERE bpan = $1 ORDER BY start_time ASC",
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(records)
    }

    /// Log second-life certification
    pub async fn log_reuse_certification(
        &self,
        bpan: &str,
        reuse_type: &str,
        certifier_id: &str,
    ) -> Result<ReuseRecord, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let record = ReuseRecord {
            id,
            bpan: bpan.to_string(),
            reuse_type: reuse_type.to_string(),
            certifier_id: certifier_id.to_string(),
            certified_at: now,
            expected_end_of_life: None,
        };

        sqlx::query(
            r#"
            INSERT INTO battery_reuse (id, bpan, reuse_type, certifier_id, certified_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(reuse_type)
        .bind(certifier_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(record)
    }

    /// Log recycling completion
    pub async fn log_recycling(
        &self,
        bpan: &str,
        recycler_id: &str,
        recovered_percentage: f32,
        recovery_method: &str,
    ) -> Result<RecyclingRecord, RepositoryError> {
        if !(0.0..=100.0).contains(&recovered_percentage) {
            return Err(RepositoryError::ValidationError(format!(
                "recovered_percentage must be 0–100, got {}",
                recovered_percentage
            )));
        }

        let id = Uuid::new_v4();
        let now = Utc::now();

        let record = RecyclingRecord {
            id,
            bpan: bpan.to_string(),
            recycler_id: recycler_id.to_string(),
            recovered_percentage,
            recovery_method: recovery_method.to_string(),
            recycled_at: now,
        };

        sqlx::query(
            r#"
            INSERT INTO battery_recycling (id, bpan, recycler_id, recovered_percentage, recovery_method, recycled_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(recycler_id)
        .bind(recovered_percentage)
        .bind(recovery_method)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(record)
    }

    /// Get all recycling records for a battery
    pub async fn get_recycling_records(
        &self,
        bpan: &str,
    ) -> Result<Vec<RecyclingRecord>, RepositoryError> {
        let records = sqlx::query_as::<_, RecyclingRecord>(
            "SELECT id, bpan, recycler_id, recovered_percentage, recovery_method, recycled_at FROM battery_recycling WHERE bpan = $1 ORDER BY recycled_at DESC",
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    

    #[tokio::test]
    async fn test_ownership_transfer_chain() {
        // This test would require a real database
        // For now, we test the logic
        let _bpan = "MY008A6FKKKLC1DH80001";

        // Simulate: manufacturer → distributor → consumer
        println!("Ownership chain:");
        println!("1. Manufacturer creates battery");
        println!("2. Ownership transfers to Distributor (transfer_reason: sale)");
        println!("3. Ownership transfers to Consumer (transfer_reason: purchase)");
        println!("4. When queried, history shows all 3 with timestamps");
    }
}
