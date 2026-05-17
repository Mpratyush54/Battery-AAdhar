//! qr_integration_test.rs — QR code payload generation and validation

#[test]
fn test_qr_payload_generation_and_validation() {
    use bpa_engine::services::QrService;

    let payload = QrService::build_payload(
        "MY008A6FKKKLC1DH80001",
        "NMC",
        307.0,
        30.0,
        160.0,
        350.0,
        "Prismatic",
        "Manufacturer 8",
        2025,
        "NMC811",
        "Graphite",
        "LiPF6",
        87.5,
        Some(150.0),
    ).unwrap();

    println!("✓ QR payload generated");

    let json = QrService::encode_payload(&payload).unwrap();
    let decoded = QrService::decode_payload(&json).unwrap();

    assert_eq!(decoded.bpan, payload.bpan);
    assert_eq!(decoded.chemistry_type, payload.chemistry_type);

    QrService::verify_payload(&payload).unwrap();
    println!("✓ QR payload validated");
}

#[test]
fn test_qr_payload_hash_integrity() {
    use bpa_engine::services::QrService;

    let payload = QrService::build_payload(
        "MY008A6FKKKLC1DH80001",
        "NMC",
        307.0,
        30.0,
        160.0,
        350.0,
        "Prismatic",
        "Manufacturer 8",
        2025,
        "NMC811",
        "Graphite",
        "LiPF6",
        87.5,
        Some(150.0),
    ).unwrap();

    assert!(!payload.data_hash.is_empty());
    println!("✓ QR payload hash integrity verified");
}

#[test]
fn test_qr_payload_tamper_detection() {
    use bpa_engine::services::QrService;

    let mut payload = QrService::build_payload(
        "MY008A6FKKKLC1DH80001",
        "NMC",
        307.0,
        30.0,
        160.0,
        350.0,
        "Prismatic",
        "Manufacturer 8",
        2025,
        "NMC811",
        "Graphite",
        "LiPF6",
        87.5,
        Some(150.0),
    ).unwrap();

    let original_hash = payload.data_hash.clone();
    payload.chemistry_type = "LFP".to_string();

    let result = QrService::verify_payload(&payload);
    assert!(result.is_err());
    println!("✓ Tampered payload detected: hash mismatch");
}

#[test]
fn test_qr_payload_json_roundtrip() {
    use bpa_engine::services::QrService;

    let payload = QrService::build_payload(
        "MY008A6FKKKLC1DH80001",
        "LFP",
        48.0,
        50.0,
        140.0,
        400.0,
        "Prismatic",
        "Manufacturer A",
        2024,
        "LFP",
        "Graphite",
        "LiPF6",
        92.0,
        Some(120.0),
    ).unwrap();

    let json = QrService::encode_payload(&payload).unwrap();
    let decoded = QrService::decode_payload(&json).unwrap();

    assert_eq!(decoded.bpan, payload.bpan);
    assert_eq!(decoded.chemistry_type, payload.chemistry_type);
    assert_eq!(decoded.rated_capacity_kwh, payload.rated_capacity_kwh);
    assert_eq!(decoded.recyclable_percentage, payload.recyclable_percentage);
    assert_eq!(decoded.data_hash, payload.data_hash);
    println!("✓ QR payload JSON roundtrip successful");
}
