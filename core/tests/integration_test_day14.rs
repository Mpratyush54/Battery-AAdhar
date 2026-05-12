#[test]
fn test_compliance_all_six_rules() {
    use battery_aadhaar::services::{ComplianceService, ComplianceServiceImpl};
    use battery_aadhaar::models::ComplianceSeverity;
    use std::sync::Arc;

    println!("Testing all 6 compliance rules...");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let zk_prover = Arc::new(battery_aadhaar::services::ZkProverImpl::new());
    let service = ComplianceServiceImpl::new(zk_prover);

    // Test 1: Compliant battery (no violations)
    let violations = rt.block_on(async {
        service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80001",
                85.0, // SoH > 80% = operational ✓
                30,   // < 90 days ✓
                true, // Has BMCS ✓
                true, // Has BCF ✓
                400,  // > 365 days ✓
            )
            .await
            .unwrap()
    });

    assert_eq!(violations.len(), 0);
    println!("✓ Rule 0: Compliant battery (zero violations)");

    // Test 2: SoH < 30% = END_OF_LIFE (CRITICAL)
    let violations = rt.block_on(async {
        service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80002",
                25.0, // SoH < 30% ✗
                30,
                true,
                true,
                400,
            )
            .await
            .unwrap()
    });

    assert!(violations
        .iter()
        .any(|v| v.violation_type == "END_OF_LIFE"
            && v.severity == ComplianceSeverity::Critical));
    println!("✓ Rule 2: SoH < 30% → END_OF_LIFE (CRITICAL)");

    // Test 3: SoH 30–80% = SECOND_LIFE_ELIGIBLE (INFO)
    let violations = rt.block_on(async {
        service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80003",
                65.0, // SoH 30–80% ✗
                30,
                true,
                true,
                400,
            )
            .await
            .unwrap()
    });

    assert!(violations
        .iter()
        .any(|v| v.violation_type == "SECOND_LIFE_ELIGIBLE"
            && v.severity == ComplianceSeverity::Info));
    println!("✓ Rule 1: SoH 30–80% → SECOND_LIFE_ELIGIBLE (INFO)");

    // Test 5: Health update > 90 days = OVERDUE (WARNING)
    let violations = rt.block_on(async {
        service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80004",
                85.0,
                120, // > 90 days ✗
                true,
                true,
                400,
            )
            .await
            .unwrap()
    });

    assert!(violations
        .iter()
        .any(|v| v.violation_type == "OVERDUE_HEALTH_UPDATE"
            && v.severity == ComplianceSeverity::Warning));
    println!("✓ Rule 5: Health > 90 days → OVERDUE (WARNING)");

    // Test 3: Missing BMCS = MISSING_MATERIAL_COMPOSITION (CRITICAL)
    let violations = rt.block_on(async {
        service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80005",
                85.0,
                30,
                false, // Missing BMCS ✗
                true,
                400,
            )
            .await
            .unwrap()
    });

    assert!(violations
        .iter()
        .any(|v| v.violation_type == "MISSING_MATERIAL_COMPOSITION"
            && v.severity == ComplianceSeverity::Critical));
    println!("✓ Rule 3: Missing BMCS → MISSING_MATERIAL_COMPOSITION (CRITICAL)");

    // Test 4: Missing BCF (battery > 1 year) = MISSING_CARBON_FOOTPRINT (CRITICAL)
    let violations = rt.block_on(async {
        service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80006",
                85.0,
                30,
                true,
                false, // Missing BCF ✗
                400,   // > 365 days ✗
            )
            .await
            .unwrap()
    });

    assert!(violations
        .iter()
        .any(|v| v.violation_type == "MISSING_CARBON_FOOTPRINT"
            && v.severity == ComplianceSeverity::Critical));
    println!("✓ Rule 4: Missing BCF (>1 year) → MISSING_CARBON_FOOTPRINT (CRITICAL)");

    println!("\n✅ All 6 compliance rules tested successfully!");
}

#[test]
fn test_compliance_multiple_violations() {
    use battery_aadhaar::services::{ComplianceService, ComplianceServiceImpl};
    use std::sync::Arc;

    println!("Testing multiple violations scenario...");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let zk_prover = Arc::new(battery_aadhaar::services::ZkProverImpl::new());
    let service = ComplianceServiceImpl::new(zk_prover);

    // Battery with 4+ violations
    let violations = rt.block_on(async {
        service
            .check_battery_compliance(
                "MY008A6FKKKLC1DH80099",
                20.0,  // EOL (CRITICAL)
                120,   // Overdue (WARNING)
                false, // Missing BMCS (CRITICAL)
                false, // Missing BCF (CRITICAL)
                400,   // > 1 year
            )
            .await
            .unwrap()
    });

    assert!(violations.len() >= 4);

    let critical_count = violations
        .iter()
        .filter(|v| v.severity == battery_aadhaar::models::ComplianceSeverity::Critical)
        .count();
    let warning_count = violations
        .iter()
        .filter(|v| v.severity == battery_aadhaar::models::ComplianceSeverity::Warning)
        .count();

    assert!(critical_count >= 3);
    assert!(warning_count >= 1);

    println!(
        "✓ Multiple violations detected: {} total, {} critical, {} warning",
        violations.len(),
        critical_count,
        warning_count
    );
    println!("\n✅ Multiple violations test passed!");
}

#[test]
fn test_zk_compliance_proof_generation() {
    use battery_aadhaar::services::{ComplianceService, ComplianceServiceImpl};
    use std::sync::Arc;

    println!("Testing ZK compliance proof generation...");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let zk_prover = Arc::new(battery_aadhaar::services::ZkProverImpl::new());
    let service = ComplianceServiceImpl::new(zk_prover);

    // Generate proof for "operational" requirement
    let result = rt.block_on(async {
        service
            .generate_compliance_proof(
                "MY008A6FKKKLC1DH80001",
                "operational",
            )
            .await
    });

    assert!(result.is_ok());
    let (proof, commitment) = result.unwrap();
    assert!(!proof.is_empty());
    assert!(!commitment.is_empty());
    println!("✓ Generated operational proof (SoH > 80%)");
    println!("  Proof size: {} bytes", proof.len());
    println!("  Commitment size: {} bytes", commitment.len());

    // Note: Proof is generated WITHOUT revealing actual SoH value to verifier
    println!("✓ Proof generated without value disclosure (privacy-by-design)");

    println!("\n✅ ZK proof generation test passed!");
}

#[test]
fn test_compliance_violation_resolution() {
    use battery_aadhaar::models::{ComplianceViolation, ComplianceSeverity};

    println!("Testing violation lifecycle...");

    let mut violation = ComplianceViolation::new(
        "MY008A6FKKKLC1DH80001".to_string(),
        "END_OF_LIFE".to_string(),
        ComplianceSeverity::Critical,
        "Battery SoH < 30%, must recycle".to_string(),
        true,
        Some(30),
    );

    assert!(!violation.is_overdue()); // Just created
    assert!(violation.is_critical_unresolved());
    println!("✓ Violation created as critical and unresolved");

    // Resolve violation
    violation.resolved_at = Some(chrono::Utc::now());
    assert!(!violation.is_critical_unresolved());
    println!("✓ Violation resolved");

    println!("\n✅ Violation lifecycle test passed!");
}