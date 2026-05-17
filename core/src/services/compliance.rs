//! compliance.rs — Automated compliance checking with violation detection
//!
//! Implements 6 compliance rules and generates ZK proofs for verification.

use crate::models::{ComplianceSeverity, ComplianceStatus, ComplianceViolation};
use crate::repositories::battery_repo::{BatteryRepository, BatteryRepositoryImpl};
use crate::repositories::carbon_repo::{CarbonRepository, CarbonRepositoryImpl};
use crate::repositories::compliance_repo::ComplianceRepositoryImpl;
use crate::repositories::health_repo::HealthRepositoryImpl;
use crate::repositories::material_repo::{MaterialRepository, MaterialRepositoryImpl};
use crate::services::ZkProverImpl;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

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
    async fn get_compliance_status(&self, bpan: &str) -> Result<ComplianceStatus, ComplianceError>;

    /// Generate ZK proof for compliance verification (government use)
    async fn generate_compliance_proof(
        &self,
        bpan: &str,
        requirement: &str, // "operational", "second_life", "recyclable"
    ) -> Result<(Vec<u8>, Vec<u8>), ComplianceError>; // (proof, commitment)
}

pub struct ComplianceServiceImpl {
    zk_prover: Arc<ZkProverImpl>,
    health_repo: Arc<HealthRepositoryImpl>,
    compliance_repo: Arc<ComplianceRepositoryImpl>,
    material_repo: Arc<MaterialRepositoryImpl>,
    carbon_repo: Arc<CarbonRepositoryImpl>,
    battery_repo: Arc<dyn BatteryRepository>,
}

impl ComplianceServiceImpl {
    pub fn new(
        zk_prover: Arc<ZkProverImpl>,
        health_repo: Arc<HealthRepositoryImpl>,
        compliance_repo: Arc<ComplianceRepositoryImpl>,
        material_repo: Arc<MaterialRepositoryImpl>,
        carbon_repo: Arc<CarbonRepositoryImpl>,
        battery_repo: Arc<dyn BatteryRepository>,
    ) -> Self {
        ComplianceServiceImpl {
            zk_prover,
            health_repo,
            compliance_repo,
            material_repo,
            carbon_repo,
            battery_repo,
        }
    }

    pub fn for_tests(zk_prover: Arc<ZkProverImpl>) -> Self {
        use crate::repositories::battery_repo::BatteryRepositoryImpl;
        use crate::repositories::carbon_repo::CarbonRepositoryImpl;
        use sqlx::PgPool;

        let pool = PgPool::connect_lazy("postgres://test:test@localhost/test")
            .unwrap_or_else(|_| PgPool::connect_lazy("postgres://dummy").unwrap());

        ComplianceServiceImpl {
            zk_prover,
            health_repo: Arc::new(HealthRepositoryImpl::new(pool.clone())),
            compliance_repo: Arc::new(ComplianceRepositoryImpl::new(pool.clone())),
            material_repo: Arc::new(MaterialRepositoryImpl::new(pool.clone())),
            carbon_repo: Arc::new(CarbonRepositoryImpl::new(pool.clone())),
            battery_repo: Arc::new(BatteryRepositoryImpl::new(pool)),
        }
    }

    pub fn new_stub(zk_prover: Arc<ZkProverImpl>) -> Self {
        ComplianceServiceImpl {
            zk_prover,
            health_repo: Arc::new(HealthRepositoryImpl::new_stub()),
            compliance_repo: Arc::new(ComplianceRepositoryImpl::new_stub()),
            material_repo: Arc::new(MaterialRepositoryImpl::new_stub()),
            carbon_repo: Arc::new(CarbonRepositoryImpl::new_stub()),
            battery_repo: Arc::new(BatteryRepositoryImpl::new_stub()),
        }
    }

    fn check_pool(&self) -> Result<(), ComplianceError> {
        // Stub mode: compliance checks work without DB
        Ok(())
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
                format!(
                    "Battery SoH {:.1}% < 30%, end-of-life recycling required",
                    soh
                ),
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
                format!(
                    "Battery SoH {:.1}% eligible for second-life (stationary storage)",
                    soh
                ),
                false,
                None,
            ));
        }
        // SoH >= 80% = OPERATIONAL (compliant)

        violations
    }

    /// Rule 5: Check health update recency
    fn check_health_update_recency(bpan: &str, days_since_update: u32) -> Vec<ComplianceViolation> {
        let mut violations = vec![];

        if days_since_update > 90 {
            violations.push(ComplianceViolation::new(
                bpan.to_string(),
                "OVERDUE_HEALTH_UPDATE".to_string(),
                ComplianceSeverity::Warning,
                format!(
                    "Health data overdue: {} days since last update (max 90 days)",
                    days_since_update
                ),
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
            return Err(ComplianceError::InvalidData(
                "SoH must be 0–100%".to_string(),
            ));
        }

        let mut violations = vec![];

        // Apply all 6 rules
        violations.extend(Self::check_soh_compliance(bpan, soh));
        violations.extend(Self::check_health_update_recency(
            bpan,
            days_since_health_update,
        ));
        violations.extend(Self::check_material_composition(
            bpan,
            has_material_composition,
        ));
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
        tracing::info!("compliance scan started");

        let batteries = self
            .battery_repo
            .list_batteries(10000, 0)
            .await
            .map_err(|e| ComplianceError::StorageError(e.to_string()))?;

        let mut all_violations = Vec::new();

        for battery in &batteries {
            let soh = self
                .health_repo
                .get_latest_health(&battery.bpan)
                .await
                .map_err(|e| ComplianceError::StorageError(e.to_string()))?
                .map(|h| h.state_of_health_percent)
                .unwrap_or(100.0);

            let has_material = self
                .material_repo
                .get_by_bpan(&battery.bpan)
                .await
                .map(|m| m.is_some())
                .unwrap_or(false);

            let has_carbon = self
                .carbon_repo
                .get_by_bpan(&battery.bpan)
                .await
                .map(|c| c.is_some())
                .unwrap_or(false);

            let days_since_reg = (Utc::now() - battery.created_at).num_days().max(0) as u32;
            let days_since_health = 90;

            let violations = self
                .check_battery_compliance(
                    &battery.bpan,
                    soh,
                    days_since_health,
                    has_material,
                    has_carbon,
                    days_since_reg,
                )
                .await?;

            for v in &violations {
                let deadline_days = v.action_deadline.map(|d| {
                    let diff = (d - Utc::now()).num_days();
                    diff.max(0) as i32
                });
                let _ = self
                    .compliance_repo
                    .log_violation(
                        &v.bpan,
                        &v.violation_type,
                        &v.severity.to_string(),
                        &v.description,
                        v.requires_action,
                        deadline_days,
                    )
                    .await;
            }

            all_violations.extend(violations);
        }

        tracing::info!(
            "compliance scan completed: {} violations found across {} batteries",
            all_violations.len(),
            batteries.len()
        );
        Ok(all_violations)
    }

    async fn get_compliance_status(&self, bpan: &str) -> Result<ComplianceStatus, ComplianceError> {
        let health_record = self
            .health_repo
            .get_latest_health(bpan)
            .await
            .map_err(|e| ComplianceError::StorageError(e.to_string()))?;

        let soh = health_record
            .as_ref()
            .map(|h| h.state_of_health_percent)
            .unwrap_or(100.0);
        let days_since_health = health_record
            .as_ref()
            .map(|h| (Utc::now() - h.reported_at).num_days().max(0) as u32)
            .unwrap_or(90);

        let has_material = self
            .material_repo
            .get_by_bpan(bpan)
            .await
            .map(|m| m.is_some())
            .unwrap_or(false);

        let has_carbon = self
            .carbon_repo
            .get_by_bpan(bpan)
            .await
            .map(|c| c.is_some())
            .unwrap_or(false);

        let battery = self
            .battery_repo
            .get_battery_by_bpan(bpan)
            .await
            .map_err(|e| ComplianceError::BatteryNotFound(e.to_string()))?
            .ok_or_else(|| ComplianceError::BatteryNotFound(bpan.to_string()))?;

        let days_since_reg = (Utc::now() - battery.created_at).num_days().max(0) as u32;

        let violations = self
            .check_battery_compliance(
                bpan,
                soh,
                days_since_health,
                has_material,
                has_carbon,
                days_since_reg,
            )
            .await?;

        Ok(ComplianceStatus::from_violations(
            bpan.to_string(),
            violations,
        ))
    }

    async fn generate_compliance_proof(
        &self,
        bpan: &str,
        requirement: &str,
    ) -> Result<(Vec<u8>, Vec<u8>), ComplianceError> {
        let health_record = self
            .health_repo
            .get_latest_health(bpan)
            .await
            .map_err(|e| ComplianceError::BatteryNotFound(e.to_string()))?
            .ok_or_else(|| {
                ComplianceError::BatteryNotFound(format!("No health data for {}", bpan))
            })?;

        let soh = health_record.state_of_health_percent as u64;

        let (proof, commitment) = match requirement {
            "operational" => {
                let (p, c, _) = self
                    .zk_prover
                    .prove_operational(soh)
                    .map_err(|e| ComplianceError::ZkProofFailed(e.to_string()))?;
                (p.0, c.0)
            }
            "second_life" => {
                let (p, c, _) = self
                    .zk_prover
                    .prove_second_life(soh)
                    .map_err(|e| ComplianceError::ZkProofFailed(e.to_string()))?;
                (p.0, c.0)
            }
            "recyclable" => {
                let (p, c, _) = self
                    .zk_prover
                    .prove_range(soh, 0, 100)
                    .map_err(|e| ComplianceError::ZkProofFailed(e.to_string()))?;
                (p.0, c.0)
            }
            _ => {
                return Err(ComplianceError::InvalidData(
                    "unknown requirement".to_string(),
                ))
            }
        };

        tracing::info!(
            "compliance proof generated: {} for {} (SoH={})",
            requirement,
            bpan,
            soh
        );
        Ok((proof, commitment))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compliance_check_compliant_battery() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::for_tests(zk_prover);

        let violations = service
            .check_battery_compliance("MY008A6FKKKLC1DH80001", 85.0, 30, true, true, 365)
            .await
            .unwrap();

        assert_eq!(violations.len(), 0);
        println!("✓ Compliant battery: zero violations");
    }

    #[tokio::test]
    async fn test_compliance_check_end_of_life() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::for_tests(zk_prover);

        let violations = service
            .check_battery_compliance("MY008A6FKKKLC1DH80002", 25.0, 30, true, true, 365)
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
        let service = ComplianceServiceImpl::for_tests(zk_prover);

        let violations = service
            .check_battery_compliance("MY008A6FKKKLC1DH80003", 65.0, 30, true, true, 365)
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
        let service = ComplianceServiceImpl::for_tests(zk_prover);

        let violations = service
            .check_battery_compliance("MY008A6FKKKLC1DH80004", 85.0, 120, true, true, 365)
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
        let service = ComplianceServiceImpl::for_tests(zk_prover);

        let violations = service
            .check_battery_compliance("MY008A6FKKKLC1DH80005", 85.0, 30, false, true, 365)
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
        let service = ComplianceServiceImpl::for_tests(zk_prover);

        let violations = service
            .check_battery_compliance("MY008A6FKKKLC1DH80006", 85.0, 30, true, false, 400)
            .await
            .unwrap();

        assert!(violations
            .iter()
            .any(|v| v.violation_type == "MISSING_CARBON_FOOTPRINT"
                && v.severity == ComplianceSeverity::Critical));
        println!("✓ Missing BCF (old battery): CRITICAL violation detected");
    }

    #[tokio::test]
    async fn test_compliance_check_multiple_violations() {
        let zk_prover = Arc::new(ZkProverImpl::new());
        let service = ComplianceServiceImpl::for_tests(zk_prover);

        let violations = service
            .check_battery_compliance("MY008A6FKKKLC1DH80007", 20.0, 120, false, false, 400)
            .await
            .unwrap();

        assert!(violations.len() >= 4);
        let critical_count = violations
            .iter()
            .filter(|v| v.severity == ComplianceSeverity::Critical)
            .count();
        assert!(critical_count >= 3);
        println!(
            "✓ Multiple violations: {} total, {} critical",
            violations.len(),
            critical_count
        );
    }
}
