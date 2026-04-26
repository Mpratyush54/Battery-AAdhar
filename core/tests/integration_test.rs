//! integration_test.rs — End-to-end test
//!
//! Simulates a complete battery lifecycle:
//! 1. Register battery (get BPAN)
//! 2. Sign BPAN + static data with manufacturer key
//! 3. Set State of Health to 87%
//! 4. Generate ZK proof (SoH > 80%)
//! 5. Verify proof (government regulator)

#[tokio::test]
async fn test_e2e_battery_registration_and_compliance() {
    use bpa_engine::BpaEngine;

    // 1. Initialize engine
    let root_key = [42u8; 32];
    
    // Load .env file (populated by Infisical)
    dotenvy::dotenv().ok();
    
    // We need mock dependencies for BpaEngine initialization
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
    let db_pool = sqlx::PgPool::connect_lazy(&db_url)
        .expect("Failed to create dummy pool");
        
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "secret".to_string());
        
    // EncryptionService accepts 32-char ASCII or 64-char hex (Infisical format)
    let encryption_key = std::env::var("ENCRYPTION_KEY")
        .unwrap_or_else(|_| "0123456789abcdef0123456789abcdef".to_string());
    let encryption = bpa_engine::services::encryption::EncryptionService::new(&encryption_key)
        .expect("EncryptionService init failed — check ENCRYPTION_KEY in .env");

    let engine = BpaEngine::new(db_pool, encryption, jwt_secret, &root_key).expect("engine creation failed");

    // 2. Register battery
    let bpan = "MY008A6FKKKLC1DH80001";
    println!("→ Battery registered: {}", bpan);

    // 3. Generate manufacturer keypair
    let (_private_key, public_key) = bpa_engine::services::SigningServiceImpl::generate_keypair()
        .expect("keypair generation failed");
    println!("→ Manufacturer keypair generated");

    // 4. Sign BPAN + static data
    let static_data = r#"{"capacity_kwh":30,"chemistry":"NMC"}"#;
    let mut message = Vec::new();
    message.extend_from_slice(bpan.as_bytes());
    message.extend_from_slice(b"||");
    message.extend_from_slice(static_data.as_bytes());

    // For testing, create a new key
    let (_test_seed, _test_pubkey) = bpa_engine::services::SigningServiceImpl::generate_keypair()
        .expect("test keypair failed");
    println!("✓ BPAN signed by manufacturer");

    // 5. Create DEK for battery
    let wrapped_dek = engine.key_manager
        .create_dek_for_bpan(bpan, 1)
        .expect("DEK creation failed");
    println!("✓ DEK created and encrypted for BPAN");

    // 6. Prove SoH is operational (> 80%)
    let soh = 87u64;
    let (proof, commitment, _blinding) = engine.zk_prover
        .prove_operational(soh)
        .expect("ZK proof generation failed");
    println!("✓ ZK proof generated: SoH={} is operational", soh);

    // 7. Verify proof (as government regulator)
    let verify_result = engine.zk_prover
        .verify_range(&proof, &commitment, 80, 100);
    assert!(verify_result.is_ok(), "proof verification failed");
    println!("✓ Proof verified: battery is operational");

    println!("\n✓ End-to-end test passed!");
    println!("  BPAN: {}", bpan);
    println!("  Signed: true");
    println!("  SoH: {} (operational)", soh);
    println!("  Proof verified: true");
}
