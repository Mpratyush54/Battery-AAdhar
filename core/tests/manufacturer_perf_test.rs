//! Performance test: batch registration of 1000 batteries
//! Verifies: < 30s, all BPANs unique, zero plaintext in DB

#[cfg(test)]
mod manufacturer_perf_tests {
    use bpa_core::services::key_manager::KeyManagerImpl;
    use bpa_core::services::manufacturer::{
        BatteryCsvRow, ManufacturerService, RegisterManufacturerRequest,
    };
    use bpa_core::services::encryption::EncryptionService;
    use sqlx::{Pool, Postgres, Row};
    use std::sync::Arc;
    use std::time::Instant;
    use uuid::Uuid;

    async fn setup() -> (Pool<Postgres>, ManufacturerService) {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://bpa:bpa@localhost:5432/bpa_test".to_string());

        let pool = Pool::<Postgres>::connect(&database_url)
            .await
            .expect("Failed to connect");

        // Clean up
        let _ = sqlx::query("DELETE FROM audit_logs WHERE action = 'BATCH_REGISTER_BATTERIES'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM battery_registration_log WHERE bpan LIKE 'PERF%'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM battery_health WHERE bpan LIKE 'PERF%'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM battery_descriptor WHERE bpan LIKE 'PERF%'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM battery_identifiers WHERE bpan LIKE 'PERF%'")
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM batteries WHERE bpan LIKE 'PERF%'")
            .execute(&pool)
            .await;

        let root_key_bytes = [99u8; 32];
        let key_manager = Arc::new(KeyManagerImpl::new(&root_key_bytes).unwrap());
        let encryption = EncryptionService::new(
            "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        )
        .unwrap();

        let service = ManufacturerService::new(pool.clone(), encryption, key_manager);
        (pool, service)
    }

    #[tokio::test]
    #[ignore = "Performance test — run manually with --ignored"]
    async fn test_batch_1000_batteries_under_30_seconds() {
        let (pool, service) = setup().await;

        let regulator_id = Uuid::new_v4();

        // Register manufacturer
        let mfr_req = RegisterManufacturerRequest {
            name: format!("TEST_Perf_MFR_{}", Uuid::new_v4()),
            country_code: "IN".to_string(),
            profile_data: "perf_test_profile".to_string(),
        };
        let mfr_resp = service
            .register_manufacturer(mfr_req, regulator_id)
            .await
            .expect("Registration failed");
        let mfr_code = mfr_resp.manufacturer_code.clone();

        // Build 1000 battery rows
        let rows: Vec<BatteryCsvRow> = (0..1000)
            .map(|i| BatteryCsvRow {
                chemistry_type: "NMC".to_string(),
                battery_category: "EV-M".to_string(),
                compliance_class: "AIS-156".to_string(),
                nominal_voltage: 48.0,
                rated_capacity_kwh: 2.5 + (i as f64 * 0.001),
                energy_density: 150.0,
                weight_kg: 25.0,
                form_factor: "PRISMATIC".to_string(),
                serial_number: format!("PF{:06}", i),
                batch_number: format!("PERF-BATCH-{:03}", i / 100),
                factory_code: "FAC-PERF".to_string(),
                production_year: 2026,
                sequence_number: format!("{:02}", i % 100),
            })
            .collect();

        let actor_id = Uuid::new_v4();

        // Measure time
        let start = Instant::now();
        let batch_resp = service
            .batch_register_batteries(mfr_resp.id, &mfr_code, rows, actor_id)
            .await
            .expect("Batch 1000 should succeed");
        let elapsed = start.elapsed();

        println!(
            "Batch 1000 registration completed in {:?} ({:.2}s)",
            elapsed,
            elapsed.as_secs_f64()
        );

        assert!(
            elapsed.as_secs() < 30,
            "Batch 1000 should complete in under 30s, took {:?}",
            elapsed
        );

        assert_eq!(batch_resp.total, 1000);
        assert_eq!(batch_resp.batteries.len(), 1000);

        // All BPANs unique
        let mut bpans: Vec<String> =
            batch_resp.batteries.iter().map(|b| b.bpan.clone()).collect();
        let orig_len = bpans.len();
        bpans.sort();
        bpans.dedup();
        assert_eq!(
            bpans.len(),
            orig_len,
            "All 1000 BPANs must be unique"
        );

        // All start with manufacturer code
        for bpan in &bpans {
            assert!(bpan.starts_with(&mfr_code));
        }

        // Zero plaintext in DB
        for bpan in &bpans {
            let row = sqlx::query(
                "SELECT encrypted_serial_number FROM battery_identifiers WHERE bpan = $1",
            )
            .bind(bpan)
            .fetch_optional(&pool)
            .await
            .unwrap();
            if let Some(row) = row {
                let enc: String = row.get("encrypted_serial_number");
                assert!(!enc.starts_with("PF"), "Serial should be encrypted in DB");
            }
        }

        println!(
            "✅ Batch 1000: all {} BPANs unique, zero plaintext in DB",
            bpans.len()
        );
    }
}
