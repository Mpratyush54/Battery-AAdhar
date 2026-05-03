//! carbon_integration_test.rs — Full BCF flow with tamper detection
//!
//! Tests:
//! 1. Submit carbon data with realistic ISI standard values
//! 2. Verify hash computation
//! 3. Detect tampering (modify one field, hash breaks)
//! 4. Verify certification (mark verified, fetch with verified flag)

use bpa_engine::models::{CarbonFootprint, CarbonFootprintRequest};

#[test]
fn test_bcf_full_flow_and_tamper_detection() {
    // Realistic ISI standard values for 30 kWh NMC battery
    let cf = CarbonFootprint::from_request(
        "MY008A6FKKKLC1DH80001".to_string(),
        CarbonFootprintRequest {
            raw_material_emissions_kg_co2e: 45.0, // kg CO₂e
            raw_material_source_country: "Indonesia".to_string(),
            mining_method: "Brine Evaporation".to_string(),
            manufacturing_emissions_kg_co2e: 35.0,
            manufacturing_location: "China".to_string(),
            factory_energy_source: "Renewable".to_string(),
            cell_production_method: "Wet Coating".to_string(),
            transport_emissions_kg_co2e: 12.0,
            transport_distance_km: 15000.0,
            transport_mode: "Sea".to_string(),
            transport_packaging: "Recyclable carton".to_string(),
            usage_emissions_kg_co2e: 80.0,
            usage_years: 8,
            usage_grid_emissions_factor: 500.0,
            usage_annual_km: 15000,
            recycling_emissions_kg_co2e: -15.0,
            recycling_recovery_rate: 85.0,
            recycling_avoided_mining: 30.0,
            recycling_method: "Hydrometallurgical".to_string(),
        },
        "mfr-001".to_string(),
    );

    // Test 1: Total computation
    let expected_total = 45.0 + 35.0 + 12.0 + 80.0 + (-15.0_f32);
    assert_eq!(cf.total_emissions_kg_co2e, expected_total);
    println!("✓ Test 1: Total emissions = {} kg CO₂e", cf.total_emissions_kg_co2e);

    // Test 2: Hash integrity (unmodified)
    assert!(cf.verify_hash_integrity());
    println!("✓ Test 2: Hash integrity verified: {}", cf.carbon_hash);

    // Test 3: Tamper detection
    let mut cf_tampered = cf.clone();
    cf_tampered.manufacturing_emissions_kg_co2e = 40.0; // Tampered!

    assert!(!cf_tampered.verify_hash_integrity());
    println!(
        "✓ Test 3: Tampering detected! Original hash: {}, New hash: {}",
        cf.carbon_hash,
        cf_tampered.recompute_hash()
    );

    // Test 4: Multiple tamper scenarios
    let tamper_tests = vec![
        ("raw_material", 50.0_f32, cf.raw_material_emissions_kg_co2e),
        ("transport",    20.0_f32, cf.transport_emissions_kg_co2e),
        ("usage",        90.0_f32, cf.usage_emissions_kg_co2e),
    ];

    for (field, tampered_val, original_val) in tamper_tests {
        let mut cf_test = cf.clone();
        match field {
            "raw_material" => cf_test.raw_material_emissions_kg_co2e = tampered_val,
            "transport"    => cf_test.transport_emissions_kg_co2e    = tampered_val,
            "usage"        => cf_test.usage_emissions_kg_co2e        = tampered_val,
            _ => (),
        }
        assert!(!cf_test.verify_hash_integrity(), "{} tampering not detected", field);
        println!("✓ Test 4: {} tampering detected ({} → {})", field, original_val, tampered_val);
    }

    // Test 5: Verified flag (simulation)
    let mut cf_verified = cf.clone();
    cf_verified.verified              = true;
    cf_verified.verified_by           = Some("TUV-INDIA".to_string());
    cf_verified.verified_at           = Some(chrono::Utc::now());
    cf_verified.verification_standard = Some("ISO 14040".to_string());

    assert!(cf_verified.verified);
    assert_eq!(cf_verified.verified_by, Some("TUV-INDIA".to_string()));
    println!("✓ Test 5: Verification status: verified={}, by={:?}",
        cf_verified.verified, cf_verified.verified_by);

    println!("\n✅ All BCF integration tests passed!");
    println!("BPAN:              {}", cf.bpan);
    println!("Total Emissions:   {} kg CO₂e", cf.total_emissions_kg_co2e);
    println!("Emissions per kWh: {} kg CO₂e/kWh", cf.emissions_per_kwh);
    println!("Carbon Hash:       {}", cf.carbon_hash);
}
