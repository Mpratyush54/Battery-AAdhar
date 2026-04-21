CREATE TABLE IF NOT EXISTS manufacturers (
            id UUID PRIMARY KEY,
            manufacturer_code VARCHAR(10) NOT NULL UNIQUE,
            name VARCHAR(255) NOT NULL,
            country_code VARCHAR(2) NOT NULL,
            encrypted_profile TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS batteries (
            bpan VARCHAR(21) PRIMARY KEY,
            manufacturer_id UUID NOT NULL,
            production_year INT NOT NULL,
            battery_category VARCHAR(50) NOT NULL,
            compliance_class VARCHAR(50) NOT NULL,
            static_hash VARCHAR(64) NOT NULL,
            carbon_hash VARCHAR(64) NOT NULL DEFAULT 'PENDING',
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS battery_identifiers (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            encrypted_serial_number TEXT NOT NULL,
            encrypted_batch_number TEXT NOT NULL,
            encrypted_factory_code TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS battery_descriptor (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            chemistry_type VARCHAR(50) NOT NULL,
            nominal_voltage DOUBLE PRECISION NOT NULL,
            rated_capacity_kwh DOUBLE PRECISION NOT NULL,
            energy_density DOUBLE PRECISION NOT NULL,
            weight_kg DOUBLE PRECISION NOT NULL,
            form_factor VARCHAR(50) NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS battery_material_composition (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            cathode_material VARCHAR(100) NOT NULL,
            anode_material VARCHAR(100) NOT NULL,
            electrolyte_type VARCHAR(100) NOT NULL,
            separator_material VARCHAR(100) NOT NULL,
            lithium_content_g DOUBLE PRECISION NOT NULL,
            cobalt_content_g DOUBLE PRECISION NOT NULL,
            nickel_content_g DOUBLE PRECISION NOT NULL,
            recyclable_percentage DOUBLE PRECISION NOT NULL,
            encrypted_details TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS carbon_footprint (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            raw_material_emission DOUBLE PRECISION NOT NULL,
            manufacturing_emission DOUBLE PRECISION NOT NULL,
            transport_emission DOUBLE PRECISION NOT NULL,
            usage_emission DOUBLE PRECISION NOT NULL,
            recycling_emission DOUBLE PRECISION NOT NULL,
            total_emission DOUBLE PRECISION NOT NULL,
            verified BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS battery_health (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            state_of_health DOUBLE PRECISION NOT NULL,
            total_cycles INT NOT NULL,
            degradation_class VARCHAR(5) NOT NULL,
            end_of_life BOOLEAN NOT NULL DEFAULT FALSE,
            updated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS ownership_history (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            encrypted_owner_identity TEXT NOT NULL,
            start_time TIMESTAMP NOT NULL,
            end_time TIMESTAMP NOT NULL DEFAULT '9999-12-31 23:59:59'
        );;

CREATE TABLE IF NOT EXISTS reuse_history (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            reuse_application VARCHAR(255) NOT NULL,
            certified_by VARCHAR(255) NOT NULL,
            certified_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS recycling_records (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            recycler_name VARCHAR(255) NOT NULL,
            recovered_material_percentage DOUBLE PRECISION NOT NULL,
            certificate_hash VARCHAR(64) NOT NULL,
            recycled_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS telemetry (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            encrypted_payload TEXT NOT NULL,
            recorded_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS qr_records (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            qr_payload_hash VARCHAR(64) NOT NULL,
            generated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS stakeholders (
            id UUID PRIMARY KEY,
            role VARCHAR(50) NOT NULL,
            encrypted_profile TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS stakeholder_credentials (
            stakeholder_id UUID PRIMARY KEY REFERENCES stakeholders(id),
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            updated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS refresh_tokens (
            id UUID PRIMARY KEY,
            stakeholder_id UUID NOT NULL REFERENCES stakeholders(id),
            token VARCHAR(255) UNIQUE NOT NULL,
            expires_at TIMESTAMP NOT NULL,
            revoked BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS stakeholder_kyc (
            stakeholder_id UUID PRIMARY KEY REFERENCES stakeholders(id),
            aadhar_number VARCHAR(12) NOT NULL,
            aadhar_document_base64 TEXT NOT NULL,
            uploaded_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS data_access_control (
            id UUID PRIMARY KEY,
            stakeholder_id UUID NOT NULL,
            resource_type VARCHAR(100) NOT NULL,
            access_level VARCHAR(50) NOT NULL,
            UNIQUE(stakeholder_id, resource_type)
        );;

CREATE TABLE IF NOT EXISTS regulator_access_log (
            id UUID PRIMARY KEY,
            stakeholder_id UUID NOT NULL,
            bpan VARCHAR(21) NOT NULL,
            reason TEXT NOT NULL,
            accessed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS audit_logs (
            id UUID PRIMARY KEY,
            actor_id UUID NOT NULL,
            action VARCHAR(100) NOT NULL,
            resource VARCHAR(255) NOT NULL,
            previous_hash VARCHAR(64) NOT NULL,
            entry_hash VARCHAR(64) NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS battery_registration_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            manufacturer_id UUID NOT NULL,
            registration_status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
            submitted_at TIMESTAMP NOT NULL DEFAULT NOW(),
            approved_at TIMESTAMP,
            approved_by UUID
        );;

CREATE TABLE IF NOT EXISTS static_data_submission_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            submitted_by UUID NOT NULL,
            data_section VARCHAR(100) NOT NULL,
            data_hash VARCHAR(64) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            submitted_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS validation_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            validation_type VARCHAR(100) NOT NULL,
            validation_result VARCHAR(50) NOT NULL,
            remarks TEXT NOT NULL DEFAULT '',
            validated_by UUID NOT NULL,
            validated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS static_data_update_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            updated_by UUID NOT NULL,
            field_name VARCHAR(100) NOT NULL,
            previous_hash VARCHAR(64) NOT NULL,
            new_hash VARCHAR(64) NOT NULL,
            updated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS dynamic_data_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            upload_type VARCHAR(50) NOT NULL,
            record_hash VARCHAR(64) NOT NULL,
            uploaded_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS ownership_transfer_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            from_owner_hash VARCHAR(64) NOT NULL,
            to_owner_hash VARCHAR(64) NOT NULL,
            transfer_reason VARCHAR(255) NOT NULL,
            transferred_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS reuse_certification_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            application_type VARCHAR(255) NOT NULL,
            certifier_hash VARCHAR(64) NOT NULL,
            certified_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS recycling_certification_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            previous_event_hash VARCHAR(64) NOT NULL,
            event_hash VARCHAR(64) NOT NULL,
            recycler_hash VARCHAR(64) NOT NULL,
            material_recovery_hash VARCHAR(64) NOT NULL,
            certified_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS qr_generation_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            payload_hash VARCHAR(64) NOT NULL,
            generated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS data_access_execution_log (
            id UUID PRIMARY KEY,
            stakeholder_id UUID NOT NULL,
            bpan VARCHAR(21) NOT NULL,
            resource_type VARCHAR(100) NOT NULL,
            access_type VARCHAR(20) NOT NULL,
            granted BOOLEAN NOT NULL,
            accessed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS compliance_violation_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            violation_type VARCHAR(50) NOT NULL,
            severity VARCHAR(20) NOT NULL,
            detected_at TIMESTAMP NOT NULL DEFAULT NOW(),
            resolved BOOLEAN NOT NULL DEFAULT FALSE
        );;

CREATE TABLE IF NOT EXISTS certificates (
            id UUID PRIMARY KEY,
            public_key TEXT NOT NULL,
            issued_by_hash VARCHAR(64) NOT NULL,
            issued_at TIMESTAMP NOT NULL DEFAULT NOW(),
            expires_at TIMESTAMP NOT NULL,
            revoked BOOLEAN NOT NULL DEFAULT FALSE
        );;

CREATE TABLE IF NOT EXISTS certificate_revocation_list (
            id UUID PRIMARY KEY,
            certificate_id UUID NOT NULL,
            revoked_by_hash VARCHAR(64) NOT NULL,
            reason VARCHAR(255) NOT NULL,
            revoked_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS root_keys (
            id UUID PRIMARY KEY,
            key_identifier VARCHAR(100) NOT NULL UNIQUE,
            hardware_backed BOOLEAN NOT NULL DEFAULT FALSE,
            status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            retired_at TIMESTAMP NOT NULL DEFAULT '9999-12-31 23:59:59'
        );;

CREATE TABLE IF NOT EXISTS kek_keys (
            id UUID PRIMARY KEY,
            encrypted_kek BYTEA NOT NULL,
            version INT NOT NULL,
            root_key_id UUID NOT NULL,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            retired_at TIMESTAMP NOT NULL DEFAULT '9999-12-31 23:59:59'
        );;

CREATE TABLE IF NOT EXISTS battery_keys (
            bpan VARCHAR(21) PRIMARY KEY REFERENCES batteries(bpan),
            encrypted_dek BYTEA NOT NULL,
            kek_version INT NOT NULL,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            key_status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            rotated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS static_signatures (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL REFERENCES batteries(bpan),
            data_section VARCHAR(100) NOT NULL,
            data_hash VARCHAR(64) NOT NULL,
            signature BYTEA NOT NULL,
            certificate_id UUID NOT NULL,
            signed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS key_rotation_log (
            id UUID PRIMARY KEY,
            key_type VARCHAR(50) NOT NULL,
            previous_version INT NOT NULL,
            new_version INT NOT NULL,
            initiated_by UUID NOT NULL,
            approved_by UUID,
            approval_timestamp TIMESTAMP,
            rotated_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS key_destruction_log (
            id UUID PRIMARY KEY,
            bpan VARCHAR(21) NOT NULL,
            dek_version INT NOT NULL,
            destroyed_by UUID NOT NULL,
            destruction_method VARCHAR(50) NOT NULL,
            destroyed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS api_requests (
            id UUID PRIMARY KEY,
            parent_request_id UUID,
            request_hash VARCHAR(64) NOT NULL,
            endpoint_hash VARCHAR(64) NOT NULL,
            subject_hash VARCHAR(64) NOT NULL,
            status_hash VARCHAR(64) NOT NULL,
            latency_ms INT NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS idempotency_keys (
            id UUID PRIMARY KEY,
            request_hash VARCHAR(64) NOT NULL UNIQUE,
            response_hash VARCHAR(64) NOT NULL,
            expires_at TIMESTAMP NOT NULL
        );;

CREATE TABLE IF NOT EXISTS rate_limits (
            id UUID PRIMARY KEY,
            subject_hash VARCHAR(64) NOT NULL,
            window_start TIMESTAMP NOT NULL,
            request_count INT NOT NULL DEFAULT 0
        );;

CREATE TABLE IF NOT EXISTS scheduled_jobs (
            id UUID PRIMARY KEY,
            job_name_hash VARCHAR(64) NOT NULL UNIQUE,
            cron_expression VARCHAR(100) NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT TRUE,
            last_run TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS job_execution_log (
            id UUID PRIMARY KEY,
            job_id UUID NOT NULL,
            status VARCHAR(20) NOT NULL,
            duration_ms INT NOT NULL DEFAULT 0,
            executed_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS notifications (
            id UUID PRIMARY KEY,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            recipient_hash VARCHAR(64) NOT NULL,
            encrypted_message TEXT NOT NULL,
            status_hash VARCHAR(64) NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS message_queue (
            id UUID PRIMARY KEY,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            topic_hash VARCHAR(64) NOT NULL,
            encrypted_payload TEXT NOT NULL,
            status_hash VARCHAR(64) NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS dead_letter_queue (
            id UUID PRIMARY KEY,
            original_message_id UUID NOT NULL,
            failure_reason_hash VARCHAR(64) NOT NULL,
            retry_count INT NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS system_metrics (
            id UUID PRIMARY KEY,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            metric_name_hash VARCHAR(64) NOT NULL,
            metric_value_cipher TEXT NOT NULL,
            recorded_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS alerts (
            id UUID PRIMARY KEY,
            cipher_algorithm VARCHAR(20) NOT NULL,
            cipher_version INT NOT NULL,
            severity_hash VARCHAR(64) NOT NULL,
            message_cipher TEXT NOT NULL,
            triggered_at TIMESTAMP NOT NULL DEFAULT NOW(),
            resolved BOOLEAN NOT NULL DEFAULT FALSE
        );;

CREATE TABLE IF NOT EXISTS regions (
            id UUID PRIMARY KEY,
            region_hash VARCHAR(64) NOT NULL,
            data_center_hash VARCHAR(64) NOT NULL
        );;

CREATE TABLE IF NOT EXISTS system_integrity_log (
            id UUID PRIMARY KEY,
            check_type VARCHAR(100) NOT NULL,
            status VARCHAR(20) NOT NULL,
            checked_at TIMESTAMP NOT NULL DEFAULT NOW()
        );;

CREATE TABLE IF NOT EXISTS data_classification (
            id UUID PRIMARY KEY,
            table_name VARCHAR(100) NOT NULL,
            field_name VARCHAR(100) NOT NULL,
            classification VARCHAR(50) NOT NULL,
            UNIQUE(table_name, field_name)
        );;

CREATE INDEX IF NOT EXISTS idx_battery_identifiers_bpan ON battery_identifiers(bpan);

CREATE INDEX IF NOT EXISTS idx_battery_descriptor_bpan ON battery_descriptor(bpan);

CREATE INDEX IF NOT EXISTS idx_battery_health_bpan ON battery_health(bpan);

CREATE INDEX IF NOT EXISTS idx_ownership_history_bpan ON ownership_history(bpan);

CREATE INDEX IF NOT EXISTS idx_telemetry_bpan ON telemetry(bpan);

CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource);

CREATE INDEX IF NOT EXISTS idx_audit_logs_actor ON audit_logs(actor_id);

CREATE INDEX IF NOT EXISTS idx_dynamic_data_log_bpan ON dynamic_data_log(bpan);

CREATE INDEX IF NOT EXISTS idx_carbon_footprint_bpan ON carbon_footprint(bpan);

CREATE INDEX IF NOT EXISTS idx_compliance_violations_bpan ON compliance_violation_log(bpan);

CREATE INDEX IF NOT EXISTS idx_recycling_records_bpan ON recycling_records(bpan);

CREATE INDEX IF NOT EXISTS idx_reuse_history_bpan ON reuse_history(bpan);

CREATE INDEX IF NOT EXISTS idx_qr_records_bpan ON qr_records(bpan);

CREATE INDEX IF NOT EXISTS idx_battery_registration_bpan ON battery_registration_log(bpan);