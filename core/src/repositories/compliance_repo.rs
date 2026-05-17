//! compliance_repo.rs — Compliance violation logging and queries

use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::Utc;
use crate::models::{ComplianceViolation, ComplianceSeverity};
use super::battery_repo::RepositoryError;

pub struct ComplianceRepositoryImpl {
    pool: Option<PgPool>,
}

impl ComplianceRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        ComplianceRepositoryImpl { pool: Some(pool) }
    }

    pub fn new_stub() -> Self {
        ComplianceRepositoryImpl { pool: None }
    }

    fn pool(&self) -> Result<&PgPool, RepositoryError> {
        self.pool.as_ref().ok_or(RepositoryError::NotFound("stub mode".to_string()))
    }

    /// Log a compliance violation
    pub async fn log_violation(
        &self,
        bpan: &str,
        violation_type: &str,
        severity: &str, // "INFO", "WARNING", "CRITICAL"
        description: &str,
        requires_action: bool,
        action_deadline_days: Option<i32>,
    ) -> Result<String, RepositoryError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let deadline = action_deadline_days.map(|d| now + chrono::Duration::days(d as i64));

        sqlx::query(
            r#"
            INSERT INTO compliance_violation_log
            (id, bpan, violation_type, severity, description, requires_action, 
             action_deadline, detected_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(bpan)
        .bind(violation_type)
        .bind(severity)
        .bind(description)
        .bind(requires_action)
        .bind(deadline)
        .bind(now)
        .execute(self.pool()?)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(id.to_string())
    }

    /// Get violations for specific battery
    pub async fn get_violations_for_battery(
        &self,
        _bpan: &str,
    ) -> Result<Vec<ComplianceViolation>, RepositoryError> {
        let rows = sqlx::query_as::<_, (uuid::Uuid, String, String, String, String, chrono::DateTime<chrono::Utc>, bool, Option<chrono::DateTime<chrono::Utc>> )>(
                r#"
                SELECT id, bpan, violation_type, severity, description, detected_at, requires_action, action_deadline
                FROM compliance_violation_log WHERE bpan = $1 AND resolved_at IS NULL
                ORDER BY detected_at DESC
                "#
            )
            .bind(_bpan)
            .fetch_all(self.pool()?)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let violations = rows.into_iter().map(|(id, bpan, violation_type, severity, description, detected_at, requires_action, action_deadline)| {
            ComplianceViolation {
                id,
                bpan,
                violation_type,
                severity: match severity.as_str() {
                    "INFO" => ComplianceSeverity::Info,
                    "WARNING" => ComplianceSeverity::Warning,
                    "CRITICAL" => ComplianceSeverity::Critical,
                    _ => ComplianceSeverity::Info,
                },
                description,
                detected_at,
                requires_action,
                action_deadline,
                resolved_at: None,
            }
        }).collect();

        Ok(violations)
    }

    /// Get all critical unresolved violations
    pub async fn get_critical_violations(
        &self,
    ) -> Result<Vec<(String, String)>, RepositoryError> {
        // Returns: (bpan, violation_type)
        let violations = sqlx::query(
            r#"
            SELECT bpan, violation_type FROM compliance_violation_log
            WHERE severity = 'CRITICAL' AND resolved_at IS NULL
            ORDER BY detected_at DESC
            "#,
        )
        .fetch_all(self.pool()?)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(violations
            .into_iter()
            .map(|v| (v.get::<String, _>("bpan"), v.get::<String, _>("violation_type")))
            .collect())
    }

    /// Get violations by severity
    pub async fn get_violations_by_severity(
        &self,
        severity: &str, // "INFO", "WARNING", "CRITICAL"
    ) -> Result<Vec<(String, String, String)>, RepositoryError> {
        // Returns: (bpan, violation_type, description)
        let violations = sqlx::query(
            "SELECT bpan, violation_type, description FROM compliance_violation_log WHERE severity = $1 AND resolved_at IS NULL",
        )
        .bind(severity)
        .fetch_all(self.pool()?)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(violations
            .into_iter()
            .map(|v| (
                v.get::<String, _>("bpan"), 
                v.get::<String, _>("violation_type"), 
                v.get::<String, _>("description")
            ))
            .collect())
    }

    /// Resolve a violation
    pub async fn resolve_violation(
        &self,
        violation_id: &str,
    ) -> Result<(), RepositoryError> {
        let now = Utc::now();

        sqlx::query("UPDATE compliance_violation_log SET resolved_at = $1 WHERE id = $2")
            .bind(now)
            .bind(violation_id)
            .execute(self.pool()?)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Count violations by severity (aggregated)
    pub async fn count_violations_by_severity(
        &self,
    ) -> Result<ComplianceStats, RepositoryError> {
        let stats = sqlx::query(
            r#"
            SELECT 
              SUM(CASE WHEN severity = 'CRITICAL' AND resolved_at IS NULL THEN 1 ELSE 0 END) as critical_count,
              SUM(CASE WHEN severity = 'WARNING' AND resolved_at IS NULL THEN 1 ELSE 0 END) as warning_count,
              SUM(CASE WHEN severity = 'INFO' THEN 1 ELSE 0 END) as info_count,
              COUNT(DISTINCT bpan) as batteries_with_violations
            FROM compliance_violation_log
            "#
        )
        .fetch_one(self.pool()?)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(ComplianceStats {
            critical_count: stats.get::<Option<i64>, _>("critical_count").unwrap_or(0) as u32,
            warning_count: stats.get::<Option<i64>, _>("warning_count").unwrap_or(0) as u32,
            info_count: stats.get::<Option<i64>, _>("info_count").unwrap_or(0) as u32,
            batteries_with_violations: stats.get::<i64, _>("batteries_with_violations") as u32,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ComplianceStats {
    pub critical_count: u32,
    pub warning_count: u32,
    pub info_count: u32,
    pub batteries_with_violations: u32,
}
