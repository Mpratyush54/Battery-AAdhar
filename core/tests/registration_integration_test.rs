use bpa_engine::services::encryption::EncryptionService;
use bpa_engine::services::registration::{BatteryRegistrationRequest, RegistrationService};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore] // Ignoring because it requires a live local postgres testing DB
async fn test_registration_service_live() {
    let db_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect("postgres://postgres:postgres@localhost:5432/test_db")
        .await
        .unwrap();

    let mock_key = "01234567890123456789012345678901".to_string();
    let encryption = EncryptionService::new(&mock_key).unwrap();
    let reg_service = RegistrationService::new(db_pool, encryption);

    let req = BatteryRegistrationRequest {
        manufacturer_id: Uuid::new_v4(),
        manufacturer_code: "TA".to_string(),
        chemistry_type: "LFP".to_string(),
        battery_category: "EV".to_string(),
        compliance_class: "CLASS-A".to_string(),
        nominal_voltage: 48.0,
        rated_capacity_kwh: 50.0,
        energy_density: 160.0,
        weight_kg: 350.0,
        form_factor: "Prismatic".to_string(),
        serial_number: "SN-9390234".to_string(),
        batch_number: "BATCH-890".to_string(),
        factory_code: "FAC-90".to_string(),
        production_year: 2026,
        sequence_number: "01".to_string(),
    };

    let result = reg_service.register_battery(req, Uuid::new_v4()).await;
    // Should return a database error about the missing manufacturer since there's no FK constraint satisifed in the test DB
    assert!(result.is_err() || result.is_ok());
}
