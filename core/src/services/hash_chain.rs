use sha2::{Digest, Sha256};
use tracing::{debug, instrument};

use crate::errors::{BpaError, BpaResult};

/// Computes SHA-256 hashes and maintains tamper-evident hash chains
/// for the audit trail as required by the BPA guidelines.
pub struct HashChainService;

impl HashChainService {
    /// Compute a SHA-256 hash of an arbitrary string and return it as a hex string.
    #[instrument(name = "compute_hash", skip(data))]
    pub fn compute_hash(data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Compute the entry hash for an audit log entry.
    /// The entry_hash = SHA256(previous_hash + action + resource + actor_id + timestamp).
    /// This creates a tamper-evident chain where modifying any prior entry
    /// invalidates all subsequent hashes.
    #[instrument(name = "compute_entry_hash", skip_all)]
    pub fn compute_entry_hash(
        previous_hash: &str,
        action: &str,
        resource: &str,
        actor_id: &str,
        timestamp: &str,
    ) -> String {
        let payload = format!(
            "{}|{}|{}|{}|{}",
            previous_hash, action, resource, actor_id, timestamp
        );
        debug!("Computing entry hash for payload length: {}", payload.len());
        Self::compute_hash(&payload)
    }

    /// Compute the static data hash for a battery's immutable attributes.
    /// This is stored in `batteries.static_hash` and signed via `static_signatures`.
    pub fn compute_static_hash(
        bpan: &str,
        chemistry_type: &str,
        nominal_voltage: f64,
        rated_capacity_kwh: f64,
        form_factor: &str,
    ) -> String {
        let payload = format!(
            "{}|{}|{:.6}|{:.6}|{}",
            bpan, chemistry_type, nominal_voltage, rated_capacity_kwh, form_factor
        );
        Self::compute_hash(&payload)
    }

    /// Compute the carbon footprint hash for integrity verification.
    pub fn compute_carbon_hash(
        bpan: &str,
        raw_material: f64,
        manufacturing: f64,
        transport: f64,
        usage: f64,
        recycling: f64,
    ) -> String {
        let payload = format!(
            "{}|{:.6}|{:.6}|{:.6}|{:.6}|{:.6}",
            bpan, raw_material, manufacturing, transport, usage, recycling
        );
        Self::compute_hash(&payload)
    }

    /// Verify that a given hash matches the expected hash for the provided data.
    pub fn verify_hash(data: &str, expected_hash: &str) -> BpaResult<()> {
        let computed = Self::compute_hash(data);
        if computed != expected_hash {
            return Err(BpaError::IntegrityViolation(format!(
                "Hash mismatch: expected {}, computed {}",
                expected_hash, computed
            )));
        }
        Ok(())
    }

    /// Verify the integrity of an audit chain by recomputing each entry's hash
    /// against its stored previous_hash + payload.
    pub fn verify_chain(entries: &[(String, String, String, String, String, String)]) -> BpaResult<()> {
        // entries: Vec<(entry_hash, previous_hash, action, resource, actor_id, timestamp)>
        for (i, (stored_hash, prev_hash, action, resource, actor, ts)) in entries.iter().enumerate() {
            let computed = Self::compute_entry_hash(prev_hash, action, resource, actor, ts);
            if computed != *stored_hash {
                return Err(BpaError::IntegrityViolation(format!(
                    "Audit chain broken at entry {}: expected {}, computed {}",
                    i, stored_hash, computed
                )));
            }
        }
        Ok(())
    }

    /// Get the genesis hash (the first "previous_hash" in any chain).
    pub fn genesis_hash() -> String {
        "0000000000000000000000000000000000000000000000000000000000000000".to_string()
    }
}
