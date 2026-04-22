# Schema audit — Day 1
Auditor: R2
Date: 2026-04-21
Source: dbschma.txt (root of repo)

## Table count
- dbschma.txt tables: 48
- README.md claim: 43
- Discrepancy: 5 extra tables

## Extra tables (not mentioned in README "43 tables" claim)
| Table | Notes |
|-------|-------|
| `kek_keys` | Key-encryption key hierarchy — needed, keep |
| `root_keys` | Root KMS entry — needed, keep |
| `battery_keys` | Per-BPAN DEK store — needed, keep |
| `static_signatures` | Ed25519 sig store — needed, keep |
| `key_destruction_log` | Key EOL audit — needed, keep |

**Recommendation:** Update README to say 48 tables. All 5 extras are load-bearing
for the ZK/encryption architecture. Do NOT remove them.

## Field type audit vs model files

### batteries table
| Schema field | Schema type | Model type | Match? |
|---|---|---|---|
| bpan | varchar | String | ✅ |
| manufacturer_id | uuid | Uuid | ✅ |
| production_year | int | i32 | ✅ |
| battery_category | varchar | String | ✅ |
| compliance_class | varchar | String | ✅ |
| static_hash | varchar | String | ✅ |
| carbon_hash | varchar | String | ✅ |
| created_at | timestamp | DateTime<Utc> | ✅ |

### battery_identifiers table
| Schema field | Schema type | Model type | Match? |
|---|---|---|---|
| id | uuid | Uuid | ✅ |
| bpan | varchar | String | ✅ |
| cipher_algorithm | varchar | String | ✅ |
| cipher_version | int | i32 | ✅ |
| encrypted_serial_number | text | String | ✅ |
| encrypted_batch_number | text | String | ✅ |
| encrypted_factory_code | text | String | ✅ |
| created_at | timestamp | DateTime<Utc> | ✅ |

### battery_keys table (KEY TABLE — verify carefully)
| Schema field | Schema type | Model type | Match? |
|---|---|---|---|
| bpan | varchar (pk) | String | ✅ |
| encrypted_dek | bytea | Vec<u8> | ✅ |
| kek_version | int | i32 | ✅ |
| cipher_algorithm | varchar | String | ✅ |
| cipher_version | int | i32 | ✅ |
| key_status | varchar | String | ✅ |
| created_at | timestamp | DateTime<Utc> | ✅ |
| rotated_at | timestamp | Option<DateTime<Utc>> | ✅ |

### kek_keys table
| Schema field | Schema type | Model type | Match? |
|---|---|---|---|
| id | uuid | Uuid | ✅ |
| encrypted_kek | bytea | Vec<u8> | ✅ |
| version | int | i32 | ✅ |
| root_key_id | uuid | Uuid | ✅ |
| cipher_algorithm | varchar | String | ✅ |
| cipher_version | int | i32 | ✅ |
| status | varchar | String | ✅ |
| created_at | timestamp | DateTime<Utc> | ✅ |
| retired_at | timestamp | Option<DateTime<Utc>> | ✅ |

## Mismatches found
0 mismatches found

## Model-to-Database Mapping Validation

### Model files count
- Total model files in `core/src/models/`: **48** (excluding mod.rs)
- Schema tables: **48**
- **Perfect 1:1 alignment** ✅

### Model validation results
| Model File | Schema Table | Status |
|---|---|---|
| alerts.rs | alerts | ✅ Mapped |
| api_requests.rs | api_requests | ✅ Mapped |
| audit_logs.rs | audit_logs | ✅ Mapped |
| batteries.rs | batteries | ✅ Mapped |
| battery_descriptor.rs | battery_descriptor | ✅ Mapped |
| battery_health.rs | battery_health | ✅ Mapped |
| battery_identifiers.rs | battery_identifiers | ✅ Mapped |
| battery_keys.rs | battery_keys | ✅ Mapped |
| battery_material_composition.rs | battery_material_composition | ✅ Mapped |
| battery_registration_log.rs | battery_registration_log | ✅ Mapped |
| carbon_footprint.rs | carbon_footprint | ✅ Mapped |
| certificates.rs | certificates | ✅ Mapped |
| certificate_revocation_list.rs | certificate_revocation_list | ✅ Mapped |
| compliance_violation_log.rs | compliance_violation_log | ✅ Mapped |
| data_access_control.rs | data_access_control | ✅ Mapped |
| data_access_execution_log.rs | data_access_execution_log | ✅ Mapped |
| data_classification.rs | data_classification | ✅ Mapped |
| dead_letter_queue.rs | dead_letter_queue | ✅ Mapped |
| dynamic_data_log.rs | dynamic_data_log | ✅ Mapped |
| idempotency_keys.rs | idempotency_keys | ✅ Mapped |
| job_execution_log.rs | job_execution_log | ✅ Mapped |
| kek_keys.rs | kek_keys | ✅ Mapped |
| key_destruction_log.rs | key_destruction_log | ✅ Mapped |
| key_rotation_log.rs | key_rotation_log | ✅ Mapped |
| manufacturers.rs | manufacturers | ✅ Mapped |
| message_queue.rs | message_queue | ✅ Mapped |
| notifications.rs | notifications | ✅ Mapped |
| ownership_history.rs | ownership_history | ✅ Mapped |
| ownership_transfer_log.rs | ownership_transfer_log | ✅ Mapped |
| qr_generation_log.rs | qr_generation_log | ✅ Mapped |
| qr_records.rs | qr_records | ✅ Mapped |
| rate_limits.rs | rate_limits | ✅ Mapped |
| recycling_certification_log.rs | recycling_certification_log | ✅ Mapped |
| recycling_records.rs | recycling_records | ✅ Mapped |
| regions.rs | regions | ✅ Mapped |
| regulator_access_log.rs | regulator_access_log | ✅ Mapped |
| reuse_certification_log.rs | reuse_certification_log | ✅ Mapped |
| reuse_history.rs | reuse_history | ✅ Mapped |
| root_keys.rs | root_keys | ✅ Mapped |
| scheduled_jobs.rs | scheduled_jobs | ✅ Mapped |
| stakeholders.rs | stakeholders | ✅ Mapped |
| static_data_submission_log.rs | static_data_submission_log | ✅ Mapped |
| static_data_update_log.rs | static_data_update_log | ✅ Mapped |
| static_signatures.rs | static_signatures | ✅ Mapped |
| system_integrity_log.rs | system_integrity_log | ✅ Mapped |
| system_metrics.rs | system_metrics | ✅ Mapped |
| telemetry.rs | telemetry | ✅ Mapped |
| validation_log.rs | validation_log | ✅ Mapped |

### Orphaned models
**None found.** All 48 model files have corresponding database tables. ✅

## Action items
- [x] ~~Update README.md: 43 → 48 tables~~ **COMPLETED**
- [x] ~~Confirm all 48 model files in `core/src/models/` have a corresponding table~~ **COMPLETED — Perfect alignment**
- [x] ~~List any model files with NO matching table~~ **COMPLETED — No orphaned models found**
