//! compliance.rs — Compliance violation models and rules

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Duration, Utc};

/// Severity levels for compliance violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceSeverity {
    Info,     // Advisory only
    Warning,  // Action needed within 14+ days
    Critical, // Action required immediately (< 7 days)
}

impl ComplianceSeverity {
    pub fn to_string(&self) -> String {
        match self {
            ComplianceSeverity::Info => "INFO".to_string(),
            ComplianceSeverity::Warning => "WARNING".to_string(),
            ComplianceSeverity::Critical => "CRITICAL".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "INFO" => Some(ComplianceSeverity::Info),
            "WARNING" => Some(ComplianceSeverity::Warning),
            "CRITICAL" => Some(ComplianceSeverity::Critical),
            _ => None,
        }
    }
}

/// Single compliance violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub id: Uuid,
    pub bpan: String,
    pub violation_type: String, // "SECOND_LIFE_ELIGIBLE", "END_OF_LIFE", "MISSING_BMCS", etc.
    pub severity: ComplianceSeverity,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub requires_action: bool,
    pub action_deadline: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl ComplianceViolation {
    pub fn new(
        bpan: String,
        violation_type: String,
        severity: ComplianceSeverity,
        description: String,
        requires_action: bool,
        deadline_days: Option<u32>,
    ) -> Self {
        let action_deadline = deadline_days.map(|d| Utc::now() + Duration::days(d as i64));

        ComplianceViolation {
            id: Uuid::new_v4(),
            bpan,
            violation_type,
            severity,
            description,
            detected_at: Utc::now(),
            requires_action,
            action_deadline,
            resolved_at: None,
        }
    }

    /// Check if violation is overdue (deadline passed)
    pub fn is_overdue(&self) -> bool {
        if let Some(deadline) = self.action_deadline {
            Utc::now() > deadline && self.resolved_at.is_none()
        } else {
            false
        }
    }

    /// Check if violation is critical and unresolved
    pub fn is_critical_unresolved(&self) -> bool {
        self.severity == ComplianceSeverity::Critical && self.resolved_at.is_none()
    }
}

/// Compliance status of a battery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub bpan: String,
    pub status: String, // "COMPLIANT", "WARNINGS_EXIST", "VIOLATIONS_EXIST"
    pub violations: Vec<ComplianceViolation>,
    pub critical_count: u32,
    pub warning_count: u32,
    pub last_checked_at: DateTime<Utc>,
}

impl ComplianceStatus {
    pub fn from_violations(bpan: String, violations: Vec<ComplianceViolation>) -> Self {
        let critical_count = violations
            .iter()
            .filter(|v| v.severity == ComplianceSeverity::Critical && v.resolved_at.is_none())
            .count() as u32;

        let warning_count = violations
            .iter()
            .filter(|v| v.severity == ComplianceSeverity::Warning && v.resolved_at.is_none())
            .count() as u32;

        let status = if critical_count > 0 {
            "VIOLATIONS_EXIST".to_string()
        } else if warning_count > 0 {
            "WARNINGS_EXIST".to_string()
        } else {
            "COMPLIANT".to_string()
        };

        ComplianceStatus {
            bpan,
            status,
            violations,
            critical_count,
            warning_count,
            last_checked_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_violation_creation() {
        let violation = ComplianceViolation::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            "END_OF_LIFE".to_string(),
            ComplianceSeverity::Critical,
            "Battery SoH < 30%, must be recycled".to_string(),
            true,
            Some(30),
        );

        assert_eq!(violation.violation_type, "END_OF_LIFE");
        assert_eq!(violation.severity, ComplianceSeverity::Critical);
        assert!(violation.requires_action);
        assert!(violation.action_deadline.is_some());
    }

    #[test]
    fn test_compliance_severity_to_string() {
        assert_eq!(ComplianceSeverity::Info.to_string(), "INFO");
        assert_eq!(ComplianceSeverity::Warning.to_string(), "WARNING");
        assert_eq!(ComplianceSeverity::Critical.to_string(), "CRITICAL");
    }

    #[test]
    fn test_compliance_status_from_violations() {
        let violations = vec![
            ComplianceViolation::new(
                "MY008A6FKKKLC1DH80001".to_string(),
                "END_OF_LIFE".to_string(),
                ComplianceSeverity::Critical,
                "SoH < 30%".to_string(),
                true,
                Some(30),
            ),
        ];

        let status = ComplianceStatus::from_violations(
            "MY008A6FKKKLC1DH80001".to_string(),
            violations,
        );

        assert_eq!(status.status, "VIOLATIONS_EXIST");
        assert_eq!(status.critical_count, 1);
    }
}
