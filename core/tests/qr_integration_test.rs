//! qr_integration_test.rs — QR code payload generation and validation

#[test]
fn test_qr_payload_generation_and_validation() {
    use battery_aadhaar::services::{QrService, QrServiceImpl};

    let rt = tokio::runtime::Runtime::new().unwrap();

    let payload_json = rt.block_on(async {
        let service = QrServiceImpl;
        service.generate_qr_payload(
            "MY008A6FKKKLC1DH80001",
            "NMC",
            30.0,
            87.5,
            vec![],
            vec![],
        ).await.unwrap()
    });

    println!("✓ QR payload generated");

    let is_valid = rt.block_on(async {
        let service = QrServiceImpl;
        service.validate_qr_payload(&payload_json).await.unwrap()
    });

    assert!(is_valid);
    println!("✓ QR payload validated");
}

#[test]
fn test_qr_payload_hash_integrity() {
    use battery_aadhaar::services::qr_service::QrPayload;

    let payload = QrPayload::new(
        "MY008A6FKKKLC1DH80001".to_string(),
        "NMC".to_string(),
        30.0,
        87.5,
    );

    assert_eq!(payload.public_fields_hash, payload.compute_hash());
    println!("✓ QR payload hash integrity verified");
}

#[test]
fn test_qr_payload_tamper_detection() {
    use battery_aadhaar::services::qr_service::QrPayload;

    let mut payload = QrPayload::new(
        "MY008A6FKKKLC1DH80001".to_string(),
        "NMC".to_string(),
        30.0,
        87.5,
    );

    let original_hash = payload.public_fields_hash.clone();

    payload.chemistry_type = "LFP".to_string();

    let new_hash = payload.compute_hash();
    assert_ne!(original_hash, new_hash);
    println!("✓ Tampered payload detected: hash mismatch");
}

#[test]
fn test_qr_payload_json_roundtrip() {
    use battery_aadhaar::services::qr_service::QrPayload;

    let payload = QrPayload::new(
        "MY008A6FKKKLC1DH80001".to_string(),
        "LFP".to_string(),
        50.0,
        92.0,
    );

    let json = payload.to_json_string();
    let decoded = QrPayload::from_json_string(&json).unwrap();

    assert_eq!(decoded.bpan, payload.bpan);
    assert_eq!(decoded.chemistry_type, payload.chemistry_type);
    assert_eq!(decoded.capacity_kwh, payload.capacity_kwh);
    assert_eq!(decoded.recyclable_percent, payload.recyclable_percent);
    assert_eq!(decoded.public_fields_hash, payload.public_fields_hash);
    println!("✓ QR payload JSON roundtrip successful");
}
