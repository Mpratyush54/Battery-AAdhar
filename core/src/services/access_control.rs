use chrono::Utc;
use sqlx::{Pool, Postgres};
use tracing::{info, warn, instrument};
use uuid::Uuid;

use crate::errors::BpaResult;
use crate::services::validation::ValidationService;

/// Manages role-based access control (RBAC) for the BPA platform.
///
/// Per BPA guidelines, different stakeholders have different access levels:
/// - **Manufacturers/Importers**: Read/Write to their own batteries
/// - **OEMs**: Read/Write to usage data for their vehicles
/// - **Service Providers**: Read battery health, limited write
/// - **Recyclers**: Read/Write to recycling data
/// - **Regulators**: Read-all access for compliance audits
/// - **Consumers**: Read basic info (QR code level)
#[derive(Clone)]
pub struct AccessControlService {
    pool: Pool<Postgres>,
}

impl AccessControlService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Check if a stakeholder has the required access to a resource.
    #[instrument(name = "check_permission", skip(self))]
    pub async fn check_permission(
        &self,
        stakeholder_id: Uuid,
        resource_type: &str,
        required_level: &str,
    ) -> BpaResult<bool> {
        // Query the data_access_control table
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT access_level FROM data_access_control WHERE stakeholder_id = $1 AND resource_type = $2"
        )
        .bind(&stakeholder_id)
        .bind(resource_type)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((level,)) => {
                let has_access = Self::level_satisfies(&level, required_level);
                if !has_access {
                    warn!(
                        "Stakeholder {} denied {} access to {} (has {})",
                        stakeholder_id, required_level, resource_type, level
                    );
                }
                Ok(has_access)
            }
            None => {
                warn!(
                    "Stakeholder {} has no access control entry for {}",
                    stakeholder_id, resource_type
                );
                Ok(false)
            }
        }
    }

    /// Grant access to a stakeholder for a resource type.
    #[instrument(name = "grant_access", skip(self))]
    pub async fn grant_access(
        &self,
        stakeholder_id: Uuid,
        resource_type: &str,
        access_level: &str,
        granter_id: Uuid,
    ) -> BpaResult<Uuid> {
        ValidationService::validate_access_level(access_level)?;

        let id = Uuid::new_v4();

        // Upsert: if entry exists, update; otherwise insert
        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM data_access_control WHERE stakeholder_id = $1 AND resource_type = $2"
        )
        .bind(&stakeholder_id)
        .bind(resource_type)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((existing_id,)) = existing {
            sqlx::query("UPDATE data_access_control SET access_level = $1 WHERE id = $2")
                .bind(access_level)
                .bind(&existing_id)
                .execute(&self.pool)
                .await?;
            info!(
                "Access updated: stakeholder {} now has {} on {}",
                stakeholder_id, access_level, resource_type
            );
            Ok(existing_id)
        } else {
            sqlx::query("INSERT INTO data_access_control (id, stakeholder_id, resource_type, access_level) VALUES ($1, $2, $3, $4)")
                .bind(&id)
                .bind(&stakeholder_id)
                .bind(resource_type)
                .bind(access_level)
                .execute(&self.pool)
                .await?;
            info!(
                "Access granted: stakeholder {} gets {} on {}",
                stakeholder_id, access_level, resource_type
            );
            Ok(id)
        }
    }

    /// Log an access event for regulatory audit purposes.
    #[instrument(name = "log_access", skip(self))]
    pub async fn log_access(
        &self,
        stakeholder_id: Uuid,
        bpan: &str,
        reason: &str,
    ) -> BpaResult<Uuid> {
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();

        sqlx::query("INSERT INTO regulator_access_log (id, stakeholder_id, bpan, reason, accessed_at) VALUES ($1, $2, $3, $4, $5)")
            .bind(&id)
            .bind(&stakeholder_id)
            .bind(bpan)
            .bind(reason)
            .bind(&now)
            .execute(&self.pool)
            .await?;

        info!("Access logged: stakeholder {} accessed BPAN {} ({})", stakeholder_id, bpan, reason);
        Ok(id)
    }

    /// Revoke all access for a stakeholder.
    pub async fn revoke_all_access(&self, stakeholder_id: Uuid) -> BpaResult<u64> {
        let result = sqlx::query("DELETE FROM data_access_control WHERE stakeholder_id = $1")
            .bind(&stakeholder_id)
            .execute(&self.pool)
            .await?;

        info!(
            "Revoked {} access entries for stakeholder {}",
            result.rows_affected(),
            stakeholder_id
        );
        Ok(result.rows_affected())
    }

    /// Set up default access for a stakeholder based on their role.
    pub async fn setup_role_defaults(
        &self,
        stakeholder_id: Uuid,
        role: &str,
        granter_id: Uuid,
    ) -> BpaResult<()> {
        ValidationService::validate_stakeholder_role(role)?;

        let grants = match role.to_uppercase().as_str() {
            "MANUFACTURER" | "IMPORTER" => vec![
                ("BATTERIES", "WRITE"),
                ("BATTERY_IDENTIFIERS", "WRITE"),
                ("BATTERY_DESCRIPTOR", "WRITE"),
                ("BATTERY_MATERIAL_COMPOSITION", "WRITE"),
                ("CARBON_FOOTPRINT", "WRITE"),
                ("QR_RECORDS", "WRITE"),
                ("BATTERY_HEALTH", "READ"),
                ("OWNERSHIP_HISTORY", "READ"),
            ],
            "OEM" => vec![
                ("BATTERIES", "READ"),
                ("BATTERY_DESCRIPTOR", "READ"),
                ("BATTERY_HEALTH", "WRITE"),
                ("TELEMETRY", "WRITE"),
                ("OWNERSHIP_HISTORY", "WRITE"),
            ],
            "SERVICE_PROVIDER" => vec![
                ("BATTERIES", "READ"),
                ("BATTERY_DESCRIPTOR", "READ"),
                ("BATTERY_HEALTH", "WRITE"),
                ("TELEMETRY", "WRITE"),
            ],
            "RECYCLER" => vec![
                ("BATTERIES", "READ"),
                ("BATTERY_DESCRIPTOR", "READ"),
                ("BATTERY_MATERIAL_COMPOSITION", "READ"),
                ("RECYCLING_RECORDS", "WRITE"),
                ("BATTERY_HEALTH", "READ"),
            ],
            "REGULATOR" => vec![
                ("BATTERIES", "REGULATOR_READ"),
                ("BATTERY_IDENTIFIERS", "REGULATOR_READ"),
                ("BATTERY_DESCRIPTOR", "REGULATOR_READ"),
                ("BATTERY_MATERIAL_COMPOSITION", "REGULATOR_READ"),
                ("CARBON_FOOTPRINT", "REGULATOR_READ"),
                ("BATTERY_HEALTH", "REGULATOR_READ"),
                ("OWNERSHIP_HISTORY", "REGULATOR_READ"),
                ("RECYCLING_RECORDS", "REGULATOR_READ"),
                ("COMPLIANCE_VIOLATION_LOG", "REGULATOR_READ"),
                ("AUDIT_LOGS", "REGULATOR_READ"),
                ("TELEMETRY", "REGULATOR_READ"),
            ],
            "AUDITOR" => vec![
                ("AUDIT_LOGS", "AUDIT_READ"),
                ("COMPLIANCE_VIOLATION_LOG", "AUDIT_READ"),
                ("CARBON_FOOTPRINT", "WRITE"),  // Auditors can verify carbon footprint
            ],
            "CONSUMER" => vec![
                ("BATTERIES", "READ"),
                ("BATTERY_DESCRIPTOR", "READ"),
            ],
            _ => vec![],
        };

        for (resource, level) in grants {
            self.grant_access(stakeholder_id, resource, level, granter_id).await?;
        }

        info!("Role defaults set up for {} stakeholder {}", role, stakeholder_id);
        Ok(())
    }

    // --- Private helpers ---

    /// Check if an access level satisfies the required level.
    /// Hierarchy: ADMIN > WRITE > READ
    /// Special: REGULATOR_READ = can read everything, AUDIT_READ = can read audit data
    fn level_satisfies(has: &str, needs: &str) -> bool {
        match needs.to_uppercase().as_str() {
            "READ" => matches!(
                has.to_uppercase().as_str(),
                "READ" | "WRITE" | "ADMIN" | "REGULATOR_READ" | "AUDIT_READ"
            ),
            "WRITE" => matches!(has.to_uppercase().as_str(), "WRITE" | "ADMIN"),
            "ADMIN" => has.eq_ignore_ascii_case("ADMIN"),
            "REGULATOR_READ" => matches!(
                has.to_uppercase().as_str(),
                "REGULATOR_READ" | "ADMIN"
            ),
            "AUDIT_READ" => matches!(
                has.to_uppercase().as_str(),
                "AUDIT_READ" | "REGULATOR_READ" | "ADMIN"
            ),
            _ => false,
        }
    }
}
