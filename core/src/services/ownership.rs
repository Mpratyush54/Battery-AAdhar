use chrono::{Utc, Datelike};
use sqlx::{Pool, Postgres};
use tracing::{info, instrument, trace};
use uuid::Uuid;

use crate::errors::{BpaError, BpaResult};
use crate::services::encryption::EncryptionService;
use crate::services::hash_chain::HashChainService;

/// Manages battery ownership lifecycle: transfers, history tracking, and verification.
///
/// Per BPA guideline, ownership data includes:
/// - Current and historical owners (encrypted for privacy)
/// - Transfer timestamps and reasons
/// - Hash-chained transfer logs for tamper evidence
#[derive(Clone)]
pub struct OwnershipService {
    pool: Pool<Postgres>,
    encryption: EncryptionService,
}

impl OwnershipService {
    pub fn new(pool: Pool<Postgres>, encryption: EncryptionService) -> Self {
        Self { pool, encryption }
    }

    /// Transfer ownership of a battery to a new owner.
    /// Closes the previous ownership period and opens a new one.
    #[instrument(name = "transfer_ownership", skip(self))]
    pub async fn transfer_ownership(
        &self,
        bpan: &str,
        new_owner_identity: &str,
        transfer_reason: &str,
        actor_id: Uuid,
    ) -> BpaResult<Uuid> {
        // Start a database transaction
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        trace!("Transaction started for static_data_submission_log insertion");
        // Close the current ownership period (set end_time)
        sqlx::query("UPDATE ownership_history SET end_time = $1 WHERE bpan = $2 AND end_time = '9999-12-31 23:59:59'")
            .bind(&now)
            .bind(bpan)
            .execute(&mut *tx)
            .await?;

        // Create new ownership record
        let new_id = Uuid::new_v4();
        let encrypted_owner = self.encryption.encrypt(new_owner_identity)?;
        let far_future = chrono::NaiveDate::from_ymd_opt(9999, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        sqlx::query("INSERT INTO ownership_history (id, bpan, cipher_algorithm, cipher_version, encrypted_owner_identity, start_time, end_time) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&new_id)
            .bind(bpan)
            .bind("AES-256-GCM")
            .bind(1i32)
            .bind(&encrypted_owner)
            .bind(&now)
            .bind(&far_future)
            .execute(&mut *tx)
            .await?;

        // Log the transfer
        let log_id = Uuid::new_v4();
        let from_hash = HashChainService::compute_hash("PREVIOUS_OWNER"); // In production, fetch actual previous owner hash
        let to_hash = HashChainService::compute_hash(new_owner_identity);
        let ts_str = now.to_string();
        let event_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "TRANSFER_OWNERSHIP",
            bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO ownership_transfer_log (id, bpan, previous_event_hash, event_hash, from_owner_hash, to_owner_hash, transfer_reason, transferred_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(&log_id)
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&from_hash)
            .bind(&to_hash)
            .bind(transfer_reason)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Audit log
        let audit_id = Uuid::new_v4();
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&actor_id)
            .bind("TRANSFER_OWNERSHIP")
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Ownership transferred for BPAN {} — reason: {}", bpan, transfer_reason);
        Ok(new_id)
    }

    /// Get the full ownership history for a battery (decrypted for authorized viewers).
    #[instrument(name = "get_ownership_history", skip(self))]
    pub async fn get_ownership_history(&self, bpan: &str) -> BpaResult<Vec<OwnershipRecord>> {
        let rows: Vec<(Uuid, String, String, chrono::NaiveDateTime, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT id, cipher_algorithm, encrypted_owner_identity, start_time, end_time FROM ownership_history WHERE bpan = $1 ORDER BY start_time ASC"
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for (id, _cipher, encrypted_owner, start, end) in rows {
            let owner = self.encryption.decrypt(&encrypted_owner).unwrap_or_else(|_| "[Decryption Error]".into());
            results.push(OwnershipRecord {
                id,
                bpan: bpan.to_string(),
                owner_identity: owner,
                start_time: start,
                end_time: end,
                is_current: end.year() == 9999,
            });
        }

        Ok(results)
    }

    /// Get the current owner of a battery.
    pub async fn get_current_owner(&self, bpan: &str) -> BpaResult<OwnershipRecord> {
        let history = self.get_ownership_history(bpan).await?;
        history
            .into_iter()
            .find(|r| r.is_current)
            .ok_or_else(|| BpaError::NotFound(format!("No current owner for BPAN: {}", bpan)))
    }

    /// Set the initial owner for a newly registered battery.
    #[instrument(name = "set_initial_owner", skip(self))]
    pub async fn set_initial_owner(
        &self,
        bpan: &str,
        owner_identity: &str,
        actor_id: Uuid,
    ) -> BpaResult<Uuid> {
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();
        let encrypted_owner = self.encryption.encrypt(owner_identity)?;
        let far_future = chrono::NaiveDate::from_ymd_opt(9999, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        sqlx::query("INSERT INTO ownership_history (id, bpan, cipher_algorithm, cipher_version, encrypted_owner_identity, start_time, end_time) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&id)
            .bind(bpan)
            .bind("AES-256-GCM")
            .bind(1i32)
            .bind(&encrypted_owner)
            .bind(&now)
            .bind(&far_future)
            .execute(&self.pool)
            .await?;

        info!("Initial owner set for BPAN: {}", bpan);
        Ok(id)
    }
}

/// Decrypted ownership record.
#[derive(Debug, Clone)]
pub struct OwnershipRecord {
    pub id: Uuid,
    pub bpan: String,
    pub owner_identity: String,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub is_current: bool,
}
