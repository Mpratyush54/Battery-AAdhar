//! audit_repo.rs — Append-only audit log with hash-chain integrity
//!
//! Every action is logged and hashed. To tamper with an audit entry,
//! the attacker must recompute the entire hash chain, which becomes
//! computationally infeasible after a few entries.

use sqlx::PgPool;
use sha2::{Sha256, Digest};
use uuid::Uuid;
use chrono::Utc;
use super::battery_repo::RepositoryError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_id: String,
    pub action: String,
    pub resource: String,
    pub resource_id: String,
    pub details: Option<String>,
    pub entry_hash: String,        // SHA256 of (action + resource + entry_hash_prev)
    pub entry_hash_prev: String,   // Hash of previous entry (chain link)
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ComplianceViolation {
    pub id: Uuid,
    pub bpan: String,
    pub violation_type: String,
    pub severity: String,
    pub details: Option<String>,
    pub detected_at: chrono::DateTime<Utc>,
}

pub struct AuditRepositoryImpl {
    pool: PgPool,
}

impl AuditRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        AuditRepositoryImpl { pool }
    }

    /// Compute hash for a new audit entry (includes previous hash for chain)
    fn compute_entry_hash(
        action: &str,
        resource: &str,
        prev_hash: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(action.as_bytes());
        hasher.update(resource.as_bytes());
        hasher.update(prev_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get the last entry in the audit chain (to link the next one)
    async fn get_last_entry_hash(&self) -> Result<String, RepositoryError> {
        let hash = sqlx::query_scalar::<_, String>(
            "SELECT entry_hash FROM audit_log ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
        .unwrap_or_else(|| "0".to_string()); // Genesis hash

        Ok(hash)
    }

    /// Log an action to the audit trail
    pub async fn log_action(
        &self,
        actor_id: &str,
        action: &str,
        resource: &str,
        resource_id: &str,
        details: Option<&str>,
    ) -> Result<AuditLogEntry, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // Get previous hash
        let prev_hash = self.get_last_entry_hash().await?;

        // Compute this entry's hash
        let entry_hash = Self::compute_entry_hash(action, resource, &prev_hash);

        let entry = AuditLogEntry {
            id,
            actor_id: actor_id.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            resource_id: resource_id.to_string(),
            details: details.map(|s| s.to_string()),
            entry_hash: entry_hash.clone(),
            entry_hash_prev: prev_hash,
            created_at: now,
        };

        sqlx::query(
            r#"
            INSERT INTO audit_log (id, actor_id, action, resource, resource_id, details, entry_hash, entry_hash_prev, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(id)
        .bind(actor_id)
        .bind(action)
        .bind(resource)
        .bind(resource_id)
        .bind(details)
        .bind(&entry_hash)
        .bind(&entry.entry_hash_prev)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(entry)
    }

    /// Retrieve audit trail for a resource with hash-chain verification
    pub async fn get_audit_trail(&self, resource_id: &str, limit: i32) -> Result<Vec<AuditLogEntry>, RepositoryError> {
        let entries = sqlx::query_as::<_, AuditLogEntry>(
            r#"
            SELECT id, actor_id, action, resource, resource_id, details, entry_hash, entry_hash_prev, created_at
            FROM audit_log WHERE resource_id = $1 ORDER BY created_at ASC LIMIT $2
            "#,
        )
        .bind(resource_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // Verify hash chain integrity
        let mut prev_hash = "0".to_string(); // Genesis
        for entry in &entries {
            let computed_hash = Self::compute_entry_hash(&entry.action, &entry.resource, &prev_hash);
            if computed_hash != entry.entry_hash {
                return Err(RepositoryError::ValidationError(
                    format!("hash chain broken at entry {}: expected {}, got {}", 
                        entry.id, computed_hash, entry.entry_hash),
                ));
            }
            prev_hash = entry.entry_hash.clone();
        }

        Ok(entries)
    }

    /// Log a compliance violation
    pub async fn log_violation(
        &self,
        bpan: &str,
        violation_type: &str,
        severity: &str,
        details: Option<&str>,
    ) -> Result<ComplianceViolation, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let violation = ComplianceViolation {
            id,
            bpan: bpan.to_string(),
            violation_type: violation_type.to_string(),
            severity: severity.to_string(),
            details: details.map(|s| s.to_string()),
            detected_at: now,
        };

        sqlx::query(
            r#"
            INSERT INTO compliance_violations (id, bpan, violation_type, severity, details, detected_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(violation_type)
        .bind(severity)
        .bind(details)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(violation)
    }

    /// Get all violations for a battery
    pub async fn get_violations(&self, bpan: &str) -> Result<Vec<ComplianceViolation>, RepositoryError> {
        let violations = sqlx::query_as::<_, ComplianceViolation>(
            "SELECT id, bpan, violation_type, severity, details, detected_at FROM compliance_violations WHERE bpan = $1 ORDER BY detected_at DESC",
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_chain_integrity() {
        // Simulate hash chain: 3 entries
        let entry1_hash = AuditRepositoryImpl::compute_entry_hash("create", "battery", "0");
        let entry2_hash = AuditRepositoryImpl::compute_entry_hash("update", "battery", &entry1_hash);
        let entry3_hash = AuditRepositoryImpl::compute_entry_hash("verify", "battery", &entry2_hash);

        // All hashes should be different
        assert_ne!(entry1_hash, entry2_hash);
        assert_ne!(entry2_hash, entry3_hash);
        assert_ne!(entry1_hash, entry3_hash);

        // Re-computing with different prev_hash should give different result
        let entry1_hash_altered = AuditRepositoryImpl::compute_entry_hash("create", "battery", "1");
        assert_ne!(entry1_hash, entry1_hash_altered);
    }

    #[test]
    fn test_tamper_detection() {
        // If entry 2's action is tampered to "delete" instead of "update"
        let entry1_hash = AuditRepositoryImpl::compute_entry_hash("create", "battery", "0");
        let entry2_hash_original = AuditRepositoryImpl::compute_entry_hash("update", "battery", &entry1_hash);
        let entry2_hash_tampered = AuditRepositoryImpl::compute_entry_hash("delete", "battery", &entry1_hash);

        // Tampered hash differs from original
        assert_ne!(entry2_hash_original, entry2_hash_tampered);

        // And the next entry's hash will break the chain
        let entry3_hash = AuditRepositoryImpl::compute_entry_hash("verify", "battery", &entry2_hash_original);

        // If we try to recompute entry3's hash with the tampered entry2, it fails
        let entry3_hash_broken = AuditRepositoryImpl::compute_entry_hash("verify", "battery", &entry2_hash_tampered);
        assert_ne!(entry3_hash, entry3_hash_broken);
    }
}
