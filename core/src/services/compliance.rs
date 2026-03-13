use chrono::Utc;
use sqlx::{Pool, Postgres};
use tracing::{info, warn, instrument};
use uuid::Uuid;

use crate::errors::{BpaError, BpaResult};

/// Manages compliance checks and violation logging per BPA guidelines.
///
/// Compliance checks are run:
/// - On battery registration (Phase 1)
/// - On static data updates
/// - On recycling certification
/// - Periodically by regulators/auditors
#[derive(Clone)]
pub struct ComplianceService {
    pool: Pool<Postgres>,
}

impl ComplianceService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Run a full compliance check on a battery.
    /// Returns a list of findings (violations and warnings).
    #[instrument(name = "check_compliance", skip(self))]
    pub async fn check_compliance(
        &self,
        bpan: &str,
        actor_id: Uuid,
    ) -> BpaResult<ComplianceReport> {
        let mut findings = Vec::new();

        // Check 1: Battery record exists
        let battery_exists: Option<(String, String, String, String)> = sqlx::query_as(
            "SELECT battery_category, compliance_class, static_hash, carbon_hash FROM batteries WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await?;

        let (_category, _compliance_class, _static_hash, carbon_hash) = match battery_exists {
            Some(data) => data,
            None => {
                return Err(BpaError::NotFound(format!("Battery not found: {}", bpan)));
            }
        };

        // Check 2: Battery descriptor exists
        let descriptor_exists: bool = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM battery_descriptor WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_one(&self.pool)
        .await
        .map(|c| c > 0)?;

        if !descriptor_exists {
            findings.push(ComplianceFinding {
                finding_type: FindingType::Violation,
                code: "BPA-001".into(),
                message: "Battery descriptor (Phase 1) not submitted".into(),
            });
        }

        // Check 3: Battery identifiers exist
        let identifiers_exist: bool = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM battery_identifiers WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_one(&self.pool)
        .await
        .map(|c| c > 0)?;

        if !identifiers_exist {
            findings.push(ComplianceFinding {
                finding_type: FindingType::Violation,
                code: "BPA-002".into(),
                message: "Battery identifiers not registered".into(),
            });
        }

        // Check 4: Material composition (Phase 2)
        let composition_exists: bool = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM battery_material_composition WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_one(&self.pool)
        .await
        .map(|c| c > 0)?;

        if !composition_exists {
            findings.push(ComplianceFinding {
                finding_type: FindingType::Warning,
                code: "BPA-003".into(),
                message: "Material composition data (Phase 2) not submitted".into(),
            });
        }

        // Check 5: Carbon footprint (Phase 3)
        if carbon_hash == "PENDING" {
            findings.push(ComplianceFinding {
                finding_type: FindingType::Warning,
                code: "BPA-004".into(),
                message: "Carbon footprint (Phase 3) not submitted".into(),
            });
        }

        // Check 6: QR code generated
        let qr_exists: bool = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM qr_records WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_one(&self.pool)
        .await
        .map(|c| c > 0)?;

        if !qr_exists {
            findings.push(ComplianceFinding {
                finding_type: FindingType::Warning,
                code: "BPA-005".into(),
                message: "QR code not generated".into(),
            });
        }

        // Check 7: Battery health record exists and is up-to-date
        let health: Option<(f64, i32, bool)> = sqlx::query_as(
            "SELECT state_of_health, total_cycles, end_of_life FROM battery_health WHERE bpan = $1"
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((soh, _cycles, eol)) = health {
            if soh < 30.0 && !eol {
                findings.push(ComplianceFinding {
                    finding_type: FindingType::Violation,
                    code: "BPA-006".into(),
                    message: format!("SoH is {:.1}% but battery not marked as end-of-life", soh),
                });
            }
        }

        // Check 8: Registration approved
        let reg_status: Option<(String,)> = sqlx::query_as(
            "SELECT registration_status FROM battery_registration_log WHERE bpan = $1 ORDER BY submitted_at DESC LIMIT 1"
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((status,)) = reg_status {
            if status != "APPROVED" {
                findings.push(ComplianceFinding {
                    finding_type: FindingType::Warning,
                    code: "BPA-007".into(),
                    message: format!("Registration status is '{}', expected APPROVED", status),
                });
            }
        }

        // Log violations
        let now = Utc::now().naive_utc();
        for finding in &findings {
            if matches!(finding.finding_type, FindingType::Violation) {
                let violation_id = Uuid::new_v4();
                sqlx::query("INSERT INTO compliance_violation_log (id, bpan, violation_type, severity, detected_at, resolved) VALUES ($1, $2, $3, $4, $5, $6)")
                    .bind(&violation_id)
                    .bind(bpan)
                    .bind(&finding.code)
                    .bind("HIGH")
                    .bind(&now)
                    .bind(false)
                    .execute(&self.pool)
                    .await?;

                warn!("Compliance violation [{}] for BPAN {}: {}", finding.code, bpan, finding.message);
            }
        }

        let violations = findings.iter().filter(|f| matches!(f.finding_type, FindingType::Violation)).count();
        let warnings = findings.iter().filter(|f| matches!(f.finding_type, FindingType::Warning)).count();

        let overall_status = if violations > 0 {
            "NON_COMPLIANT"
        } else if warnings > 0 {
            "PARTIALLY_COMPLIANT"
        } else {
            "FULLY_COMPLIANT"
        };

        info!(
            "Compliance check for BPAN {}: {} ({} violations, {} warnings)",
            bpan, overall_status, violations, warnings
        );

        Ok(ComplianceReport {
            bpan: bpan.to_string(),
            status: overall_status.to_string(),
            violations_count: violations,
            warnings_count: warnings,
            findings,
            checked_at: now,
            checked_by: actor_id,
        })
    }

    /// Resolve a compliance violation.
    pub async fn resolve_violation(
        &self,
        violation_id: Uuid,
    ) -> BpaResult<()> {
        let result = sqlx::query("UPDATE compliance_violation_log SET resolved = true WHERE id = $1")
            .bind(&violation_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(BpaError::NotFound(format!(
                "No violation found with id {}",
                violation_id
            )));
        }

        info!("Compliance violation {} resolved", violation_id);
        Ok(())
    }

    /// Get all unresolved violations for a battery.
    pub async fn get_violations(&self, bpan: &str) -> BpaResult<Vec<ViolationRecord>> {
        let rows: Vec<(Uuid, String, String, chrono::NaiveDateTime, bool)> = sqlx::query_as(
            "SELECT id, violation_type, severity, detected_at, resolved FROM compliance_violation_log WHERE bpan = $1 ORDER BY detected_at DESC"
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id, vtype, sev, at, resolved)| ViolationRecord {
            id,
            bpan: bpan.to_string(),
            violation_type: vtype,
            severity: sev,
            detected_at: at,
            resolved,
        }).collect())
    }
}

/// Compliance check report.
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    pub bpan: String,
    pub status: String,        // FULLY_COMPLIANT, PARTIALLY_COMPLIANT, NON_COMPLIANT
    pub violations_count: usize,
    pub warnings_count: usize,
    pub findings: Vec<ComplianceFinding>,
    pub checked_at: chrono::NaiveDateTime,
    pub checked_by: Uuid,
}

/// An individual compliance finding.
#[derive(Debug, Clone)]
pub struct ComplianceFinding {
    pub finding_type: FindingType,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum FindingType {
    Violation,
    Warning,
}

/// A stored violation record.
#[derive(Debug, Clone)]
pub struct ViolationRecord {
    pub id: Uuid,
    pub bpan: String,
    pub violation_type: String,
    pub severity: String,
    pub detected_at: chrono::NaiveDateTime,
    pub resolved: bool,
}