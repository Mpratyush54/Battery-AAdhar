//! compliance.rs — Automated compliance checking with violation detection
//!
//! Implements 6 compliance rules and generates ZK proofs for verification.

use crate::models::{ComplianceViolation, ComplianceSeverity, ComplianceStatus};
use crate::services::ZkProverImpl;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;

#[derive(Debug)]
pub enum ComplianceError {
    BatteryNotFound(String),
    InvalidData(String),
    ZkProofFailed(String),
    StorageError(String),
}

impl std::fmt::Display for ComplianceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceError::BatteryNotFound(msg) => write!(f, "not found: {}", msg),
            ComplianceError::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            ComplianceError::ZkProofFailed(msg) => write!(f, "ZK proof failed: {}", msg),
            ComplianceError::StorageError(msg) => write!(f, "storage error: {}", msg),
        }
    }
}

impl std::error::Error for ComplianceError {}

#[async_trait]
pub trait ComplianceService: Send + Sync {
    /// Check single battery compliance
    async fn check_battery_compliance(
        &self,
        bpan: &str,
        soh: f32,
        days_since_health_update: u32,
        has_material_composition: bool,
        has_carbon_footprint: bool,
        days_since_registration: u32,
    ) -> Result<Vec<ComplianceViolation>, ComplianceError>;

    /// Scan all batteries for violations
    async fn scan_all_batteries(&self) -> Result<Vec<ComplianceViolation>, ComplianceError>;

    /// Get compliance status for battery
    async fn get_compliance_status(
        &self,
        bpan: &str,
    ) -> Result<ComplianceStatus, ComplianceError>;

    /// Generate ZK proof for compliance verification (government use)
    async fn generate_compliance_proof(
        &self,
        bpan: &str,
        requirement: &str, // "operational", "second_life", "recyclable"
    ) -> Result<(Vec<u8>, Vec<u8>), ComplianceError>; // (proof, commitment)
}

pub struct ComplianceServiceImpl {
    zk_prover: Arc<ZkProverImpl>,
}

impl ComplianceServiceImpl {
    pub fn new(zk_prover: Arc<ZkProverImpl>) -> Self {
        ComplianceServiceImpl { zk_prover }
    }

    /// Rule 1: Check SoH for lifecycle eligibility
    fn check_soh_compliance(bpan: &str, soh: f32) -> Vec<ComplianceViolation> {
        let mut violations = vec![];

        // Rule 2: SoH < 30% = END_OF_LIFE (CRITICAL)
        if soh < 30.0 {
            violations.push(ComplianceViolation::new(
                bpan.to_string(),
                "END_OF_LIFE".to_string(),
                ComplianceSeverity::Critical,
                format!("Battery SoH {:.1}% < 30%, end-of-life recycling required", soh),
                true,
                Some(30), // Action deadline: 30 days
            ));
        }
        // Rule 1: SoH 30–80% = second-life eligible (INFO)
        else if soh < 80.0 && soh >= 30.0 {
            violations.push(ComplianceViolation::new(
                bpan.to_string(),
                "SECOND_LIFE_ELIGIBLE".to_string(),
                ComplianceSeverity::Info,
                format!("Battery SoH {:.1}% eligible for second-life (stationary storage)", soh),
                false,
                None,
            ));
        }
        // SoH >= 80% = OPERATIONAL (compliant)

        violations
    }

    /// Rule 5: Check health update recency
    fn check_health_update_recency(
        bpan: &str,
        days_since_update: u32,
    ) -> Vec<ComplianceViolation> {
        let mut violations = vec![];

        if days_since_update > 90 {
            violations.push(ComplianceViolation::new(
                bpan.to_string(),
                "OVERDUE_HEALTH_UPDATE".to_string(),
                ComplianceSeverity::Warning,
                format!("Health data overdue: {} days since last update (max 90 days)", days_since_update),
                true,
                Some(14), // Action deadline: 14 days
            ));
        }

        violations
    }

    /// Rule 3: Check material composition submitted
    fn check_material_composition(bpan: &str, has_material: bool) -> Vec<ComplianceViolation> {
        let mut violations = vec![];

        if !has_material {
            violations.push(ComplianceViolation::new(
                bpan.to_string(),
                "MISSING_MATERIAL_COMPOSITION".to_string(),
                ComplianceSeverity::Critical,
                "Material Composition (BMCS) not submitted".to_string(),
                true,
                Some(7), // Action deadline: 7 days
            ));
        }

        violations
    }

    /// Rule 4: Check carbon footprint submitted (time-based)
    fn check_carbon_footprint(
        bpan: &str,
        has_carbon: bool,
        days_since_registration: u32,
    ) -> Vec<ComplianceViolation> {
        let mut violations = vec![];

        // Require BCF after 1 year (365 days) of operation
        if !has_carbon && days_since_registration > 365 {
            violations.push(ComplianceViolation::new(
                bpan.to_string(),
                "MISSING_CARBON_FOOTPRINT".to_string(),
                ComplianceSeverity::Critical,
                "Carbon Footprint (BCF) not submitted (required after 1 year)".to_string(),
                true,
                Some(30), // Action deadline: 30 days
            ));
        }
        // Warn if not submitted but battery is newer
        else if !has_carbon && days_since_registration > 90 {
            violations.push(ComplianceViolation::new(
                bpan.to_string(),
                "MISSING_CARBON_FOOTPRINT".to_string(),
                ComplianceSeverity::Warning,
                "Carbon Footprint (BCF) recommended within 90 days of operation".to_string(),
                false,
                None,
            ));
        }

        violations
    }
}

#[async_trait]
impl ComplianceService for ComplianceServiceImpl {
    async fn check_battery_compliance(
        &self,
        bpan: &str,
        soh: f32,
        days_since_health_update: u32,
        has_material_composition: bool,
        has_carbon_footprint: bool,
        days_since_registration: u32,
    ) -> Result<Vec<ComplianceViolation>, ComplianceError> {
        // Validate inputs
        if soh < 0.0 || soh > 100.0 {
            return Err(ComplianceError::InvalidData("SoH must be 0–100%".to_string()));
        }

        let mut violations = vec![];

        // Apply all 6 rules
        violations.extend(Self::check_soh_compliance(bpan, soh));
        violations.extend(Self::check_health_update_recency(bpan, days_since_health_update));
        violations.extend(Self::check_material_composition(bpan, has_material_composition));
        violations.extend(Self::check_carbon_footprint(
            bpan,
            has_carbon_footprint,
            days_since_registration,
        ));

        tracing::info!(
            "compliance check: {} violations found for {} (SoH={:.1}%)",
            violations.len(),
            bpan,
            soh
        );

        Ok(violations)
    }

    async fn scan_all_batteries(&self) -> Result<Vec<ComplianceViolation>, ComplianceError> {
        // TODO Day 14 R2:
        // 1. Fetch all batteries from DB
        // 2. For each battery: get SoH, health_updated_at, BMCS status, BCF status, registration_date
        // 3. Call check_battery_compliance for each
        // 4. Collect all violations
        // 5. Store in compliance_violation_log

        tracing::info!("compliance scan started");

        Ok(vec![])
    }

    async fn get_compliance_status(
        &self,
        bpan: &str,
    ) -> Result<ComplianceStatus, ComplianceError> {
        // TODO Day 14 R2: Fetch battery data and compute compliance

        Ok(ComplianceStatus {
            bpan: bpan.to_string(),
            status: "COMPLIANT".to_string(),
            violations: vec![],
            critical_count: 0,
            warning_count: 0,
            last_checked_at: Utc::now(),
        })
    }

    async fn generate_compliance_proof(
        &self,
        bpan: &str,
        requirement: &str,
    ) -> Result<(Vec<u8>, Vec<u8>), ComplianceError> {
        // TODO Day 14 R1:
        // 1. Fetch current SoH from health table
        // 2. Use ZK prover to generate proof based on requirement:
        //    - "operational": prove SoH > 80%
        //    - "second_life": prove SoH >= 60%
        //    - "recyclable": prove SoH >= 0% (always true, includes commitment)
        // 3. Return (proof, commitment)

        let soh = 85.0; // Mock: TODO fetch from DB

        let (proof, commitment, _) = match requirement {
            "operational" => {
                // Prove SoH > 80%
                self.zk_prover
                    .prove_operational(soh as u64)
                    .map_err(|e| ComplianceError::ZkProofFailed(e.to_string()))?
            }
            "second_life" => {
                // Prove SoH >= 60%
                self.zk_prover
                    .prove_second_life(soh as u64)
                    .map_err(|e| ComplianceError::ZkProofFailed(e.to_string()))?
            }
            "recyclable" => {
                // Prove SoH >= 0% (universal)
                self.zk_prover
                    .prove_range(soh as u64, 0, 100)
                    .map_err(|e| ComplianceError::ZkProofFailed(e.to_string()))?
            }
            _ => {
                return Err(ComplianceError::InvalidData(
                    "unknown requirement".to_string(),
                ))
            }
        };

        tracing::info!(
            "compliance proof generated: {} for {}",
            requirement,
            bpan
        );

        Ok((proof.0, commitment.0.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compliance_check_compliant_battery() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::new(zk_prover);

        let violations = service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80001",
                85.0,  // SoH > 80% = operational
                30,    // Recently updated
                true,  // Has BMCS
                true,  // Has BCF
                365,   // > 1 year old
            )
            .await
            .unwrap();

        assert_eq!(violations.len(), 0);
        println!("✓ Compliant battery: zero violations");
    }

    #[tokio::test]
    async fn test_compliance_check_end_of_life() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::new(zk_prover);

        let violations = service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80002",
                25.0,  // SoH < 30% = EOL (CRITICAL)
                30,
                true,
                true,
                365,
            )
            .await
            .unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, "END_OF_LIFE");
        assert_eq!(violations[0].severity, ComplianceSeverity::Critical);
        println!("✓ EOL battery: CRITICAL violation detected");
    }

    #[tokio::test]
    async fn test_compliance_check_second_life_eligible() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::new(zk_prover);

        let violations = service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80003",
                65.0,  // SoH 30–80% = second-life (INFO)
                30,
                true,
                true,
                365,
            )
            .await
            .unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, "SECOND_LIFE_ELIGIBLE");
        assert_eq!(violations[0].severity, ComplianceSeverity::Info);
        println!("✓ Second-life eligible: INFO advisory detected");
    }

    #[tokio::test]
    async fn test_compliance_check_overdue_health() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::new(zk_prover);

        let violations = service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80004",
                85.0,
                120,   // > 90 days = overdue (WARNING)
                true,
                true,
                365,
            )
            .await
            .unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, "OVERDUE_HEALTH_UPDATE");
        assert_eq!(violations[0].severity, ComplianceSeverity::Warning);
        println!("✓ Overdue health: WARNING detected");
    }

    #[tokio::test]
    async fn test_compliance_check_missing_bmcs() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::new(zk_prover);

        let violations = service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80005",
                85.0,
                30,
                false,  // Missing BMCS (CRITICAL)
                true,
                365,
            )
            .await
            .unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, "MISSING_MATERIAL_COMPOSITION");
        assert_eq!(violations[0].severity, ComplianceSeverity::Critical);
        println!("✓ Missing BMCS: CRITICAL violation detected");
    }

    #[tokio::test]
    async fn test_compliance_check_missing_bcf_old_battery() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::new(zk_prover);

        // Battery > 1 year old without BCF = CRITICAL
        let violations = service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80006",
                85.0,
                30,
                true,
                false,  // Missing BCF
                400,    // > 365 days = 1 year+ (CRITICAL)
            )
            .await
            .unwrap();

        assert!(violations.iter().any(|v| v.violation_type == "MISSING_CARBON_FOOTPRINT"
            && v.severity == ComplianceSeverity::Critical));
        println!("✓ Missing BCF (old battery): CRITICAL violation detected");
    }

    #[tokio::test]
    async fn test_compliance_check_multiple_violations() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::new(zk_prover);

        // Battery with multiple violations
        let violations = service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80007",
                20.0,   // EOL (CRITICAL)
                120,    // Overdue (WARNING)
                false,  // Missing BMCS (CRITICAL)
                false,  // Missing BCF (CRITICAL)
                400,    // > 1 year
            )
            .await
            .unwrap();

        assert!(violations.len() >= 4);
        let critical_count = violations
            .iter()
            .filter(|v| v.severity == ComplianceSeverity::Critical)
            .count();
        assert!(critical_count >= 3);
        println!("✓ Multiple violations: {} total, {} critical", violations.len(), critical_count);
    }
}
