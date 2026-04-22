//! audit_repo.rs — Immutable audit & compliance logs

use async_trait::async_trait;
use super::battery_repo::RepositoryError;

#[async_trait]
pub trait AuditRepository: Send + Sync {
        /// Log an action to the audit trail (append-only).
        async fn log_action(
                &self,
                actor_id: &str,
                action: &str,
                resource: &str,
                resource_id: &str,
        ) -> Result<String, RepositoryError>;

        /// Retrieve audit log entries for a resource.
        async fn get_audit_trail(
                &self,
                resource_id: &str,
                limit: i32,
        ) -> Result<Vec<AuditLogEntry>, RepositoryError>;

        /// Log a data access event (for compliance tracking).
        async fn log_data_access(
                &self,
                stakeholder_id: &str,
                bpan: &str,
                data_section: &str,
                purpose: &str,
        ) -> Result<(), RepositoryError>;

        /// Log a compliance violation.
        async fn log_violation(
                &self,
                bpan: &str,
                violation_type: &str,
                severity: &str,
        ) -> Result<(), RepositoryError>;

        /// Get all violations for a battery.
        async fn get_violations(
                &self,
                bpan: &str,
        ) -> Result<Vec<ComplianceViolation>, RepositoryError>;
}

pub struct AuditLogEntry {
        pub id: String,
        pub actor_id: String,
        pub action: String,
        pub resource: String,
        pub entry_hash: String,
        pub created_at: String,
}

pub struct ComplianceViolation {
        pub id: String,
        pub bpan: String,
        pub violation_type: String,
        pub severity: String,
        pub detected_at: String,
}
