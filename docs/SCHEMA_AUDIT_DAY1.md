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

## Action items
- [ ] Update README.md: 43 → 48 tables
- [ ] Confirm all 49 model files in `core/src/models/` have a corresponding table
- [ ] List any model files with NO matching table (orphaned models)
