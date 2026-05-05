//! ownership_repo.rs — Dual-party consent ownership transfers and lifecycle hash-chain storage
//!
//! This repository handles the multi-step ownership transfer process and 
//! ensures that all battery events are hash-chained for integrity.

use super::battery_repo::RepositoryError;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct OwnershipRepositoryImpl {
    pool: PgPool,
}

impl OwnershipRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        OwnershipRepositoryImpl { pool }
    }

    /// Initiate an ownership transfer
    pub async fn initiate_transfer(
        &self,
        bpan: &str,
        from_owner_id: &str,
        to_owner_id: &str,
        from_owner_role: &str,
        to_owner_role: &str,
        reason: &str,
    ) -> Result<String, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // Check if there is already a pending transfer for this battery
        let pending_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ownership_transfers WHERE bpan = $1 AND transferred_at IS NULL AND rejected = false"
        )
        .bind(bpan)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        if pending_count > 0 {
            return Err(RepositoryError::DatabaseError("pending transfer already exists for this battery".to_string()));
        }

        sqlx::query(
            r#"
            INSERT INTO ownership_transfers (
                id, bpan, from_owner_id, to_owner_id, from_owner_role, to_owner_role, 
                reason, initiated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(from_owner_id)
        .bind(to_owner_id)
        .bind(from_owner_role)
        .bind(to_owner_role)
        .bind(reason)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id.to_string())
    }

    /// Confirm a transfer (can be called by either party)
    pub async fn confirm_transfer(
        &self,
        transfer_id: &str,
        confirming_owner_id: &str,
    ) -> Result<bool, RepositoryError> {
        let transfer_id = Uuid::parse_str(transfer_id)
            .map_err(|_| RepositoryError::ValidationError("invalid transfer id".to_string()))?;

        // Fetch transfer details
        #[allow(dead_code)]
        struct TransferRow {
            from_owner_id: String,
            to_owner_id: String,
            bpan: String,
            from_owner_confirmed: bool,
            to_owner_confirmed: bool,
        }

        let transfer = sqlx::query_as::<_, (String, String, String, bool, bool)>(
            "SELECT from_owner_id, to_owner_id, bpan, from_owner_confirmed, to_owner_confirmed FROM ownership_transfers WHERE id = $1"
        )
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| RepositoryError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RepositoryError::DatabaseError("transfer not found".to_string()))?;

        // Unpack tuple
        let from_owner_id_db = &transfer.0;
        let to_owner_id_db = &transfer.1;
        let bpan_db = transfer.2.clone();

        // Determine who is confirming
        let (is_from_owner, is_to_owner) =
            (confirming_owner_id == from_owner_id_db.as_str(), confirming_owner_id == to_owner_id_db.as_str());

        if !is_from_owner && !is_to_owner {
            return Err(RepositoryError::DatabaseError("not a party to this transfer".to_string()));
        }

        let now = Utc::now();
        let mut tx = self.pool.begin()
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // Update confirmation
        if is_from_owner {
            sqlx::query("UPDATE ownership_transfers SET from_owner_confirmed = true, from_owner_confirmed_at = $1 WHERE id = $2")
                .bind(now)
                .bind(transfer_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;
        } else {
            sqlx::query("UPDATE ownership_transfers SET to_owner_confirmed = true, to_owner_confirmed_at = $1 WHERE id = $2")
                .bind(now)
                .bind(transfer_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;
        }

        // Check if both confirmed
        let both_confirmed = sqlx::query_scalar::<_, bool>(
            "SELECT from_owner_confirmed AND to_owner_confirmed FROM ownership_transfers WHERE id = $1"
        )
        .bind(transfer_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        if both_confirmed {
            // Update transferred_at timestamp
            sqlx::query(
                "UPDATE ownership_transfers SET transferred_at = $1 WHERE id = $2"
            )
            .bind(now)
            .bind(transfer_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            // Update battery's current owner
            sqlx::query(
                r#"
                UPDATE batteries 
                SET current_owner_id = $1, 
                    current_owner_role = (
                        SELECT to_owner_role FROM ownership_transfers WHERE id = $2
                    )
                WHERE bpan = $3
                "#
            )
            .bind(to_owner_id_db)
            .bind(transfer_id)
            .bind(&bpan_db)
            .execute(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| RepositoryError::DatabaseError(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(both_confirmed)
    }

    /// Reject a transfer
    pub async fn reject_transfer(
        &self,
        transfer_id: &str,
        rejecting_owner_id: &str,
        reason: &str,
    ) -> Result<(), RepositoryError> {
        let transfer_id = Uuid::parse_str(transfer_id)
            .map_err(|_| RepositoryError::ValidationError("invalid transfer id".to_string()))?;
        
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE ownership_transfers 
            SET rejected = true, rejected_by = $1, rejection_reason = $2, rejected_at = $3
            WHERE id = $4
            "#,
        )
        .bind(rejecting_owner_id)
        .bind(reason)
        .bind(now)
        .bind(transfer_id)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Record a lifecycle event in the hash-chained log
    pub async fn record_lifecycle_event(
        &self,
        bpan: &str,
        event_type: &str,
        from_state: Option<&str>,
        to_state: Option<&str>,
        actor_id: &str,
        actor_role: &str,
        details: &str,
        entry_hash: &str,
        prev_hash: &str,
    ) -> Result<(), RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO lifecycle_events (
                id, bpan, event_type, from_state, to_state, actor_id, actor_role, 
                details, entry_hash, entry_hash_prev, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(event_type)
        .bind(from_state)
        .bind(to_state)
        .bind(actor_id)
        .bind(actor_role)
        .bind(details)
        .bind(entry_hash)
        .bind(prev_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get the last event hash for a battery
    pub async fn get_last_event_hash(&self, bpan: &str) -> Result<String, RepositoryError> {
        let hash = sqlx::query_scalar::<_, String>(
            "SELECT entry_hash FROM lifecycle_events WHERE bpan = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .unwrap_or_else(|| "0".to_string());

        Ok(hash)
    }

    /// Get current owner of a battery
    pub async fn get_current_owner(&self, bpan: &str) -> Result<(String, String), RepositoryError> {
        let res = sqlx::query_as::<_, (Option<String>, Option<String>)>(
            "SELECT current_owner_id, current_owner_role FROM batteries WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| RepositoryError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RepositoryError::DatabaseError("battery not found".to_string()))?;

        Ok((
            res.0.unwrap_or_default(),
            res.1.unwrap_or_default(),
        ))
    }
}
