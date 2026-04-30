//! material.rs — Battery Material Composition Sheet (BMCS) service
//!
//! Handles submission and retrieval of material composition data.
//! Private fields (rows 22–43 per spec) are encrypted with the battery's
//! per-BPAN DEK via AES-256-GCM before persistence.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, instrument};

use crate::errors::{BpaError, BpaResult};
use crate::services::encryption::EncryptionService;

/// Full material composition record (internal representation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialComposition {
    pub bpan: String,

    // Public fields — stored in plaintext
    pub cathode_material: String,
    pub anode_material: String,
    pub electrolyte_type: String,
    pub separator_material: String,
    pub recyclable_percentage: f64,

    // Private fields — encrypted at rest
    pub lithium_content_g: f64,
    pub cobalt_content_g: f64,
    pub nickel_content_g: f64,
    pub manganese_content_g: f64,
    pub lead_content_g: f64,
    pub cadmium_content_g: f64,
    pub hazardous_substances: String,
    pub supply_chain_source: String,
}

/// Row stored in Postgres — private fields are ciphertext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCompositionRow {
    pub bpan: String,
    // Public (plaintext)
    pub cathode_material: String,
    pub anode_material: String,
    pub electrolyte_type: String,
    pub separator_material: String,
    pub recyclable_percentage: f64,
    // Private (base64 ciphertext blob)
    pub encrypted_details: String,
}

/// Subset returned to roles that may NOT see private fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCompositionPublic {
    pub bpan: String,
    pub cathode_material: String,
    pub anode_material: String,
    pub electrolyte_type: String,
    pub separator_material: String,
    pub recyclable_percentage: f64,
}

/// Private fields bundle — serialised to JSON then encrypted.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrivateFields {
    lithium_content_g: f64,
    cobalt_content_g: f64,
    nickel_content_g: f64,
    manganese_content_g: f64,
    lead_content_g: f64,
    cadmium_content_g: f64,
    hazardous_substances: String,
    supply_chain_source: String,
}

/// Roles allowed to see private material fields.
const PRIVATE_ROLES: &[&str] = &["manufacturer", "importer", "admin", "government"];

/// Service that handles BMCS operations.
#[derive(Clone)]
pub struct MaterialService {
    encryption: EncryptionService,
}

impl MaterialService {
    pub fn new(encryption: EncryptionService) -> Self {
        Self { encryption }
    }

    /// Encrypt private fields and produce a storable row + data hash.
    #[instrument(name = "material_submit", skip(self, comp), fields(bpan = %comp.bpan))]
    pub fn prepare_submission(
        &self,
        comp: &MaterialComposition,
    ) -> BpaResult<(MaterialCompositionRow, String)> {
        let private = PrivateFields {
            lithium_content_g: comp.lithium_content_g,
            cobalt_content_g: comp.cobalt_content_g,
            nickel_content_g: comp.nickel_content_g,
            manganese_content_g: comp.manganese_content_g,
            lead_content_g: comp.lead_content_g,
            cadmium_content_g: comp.cadmium_content_g,
            hazardous_substances: comp.hazardous_substances.clone(),
            supply_chain_source: comp.supply_chain_source.clone(),
        };

        let private_json = serde_json::to_string(&private)
            .map_err(|e| BpaError::Internal(format!("serialize private fields: {}", e)))?;

        let encrypted_details = self.encryption.encrypt(&private_json)?;

        // Compute SHA-256 data hash over the full composition for audit trail
        let data_hash = Self::compute_data_hash(comp);

        let row = MaterialCompositionRow {
            bpan: comp.bpan.clone(),
            cathode_material: comp.cathode_material.clone(),
            anode_material: comp.anode_material.clone(),
            electrolyte_type: comp.electrolyte_type.clone(),
            separator_material: comp.separator_material.clone(),
            recyclable_percentage: comp.recyclable_percentage,
            encrypted_details,
        };

        info!(bpan = %comp.bpan, "BMCS submission prepared (private fields encrypted)");
        Ok((row, data_hash))
    }

    /// Reconstruct full composition from a stored row (decrypt private fields).
    #[instrument(name = "material_decrypt", skip(self, row), fields(bpan = %row.bpan))]
    pub fn decrypt_row(&self, row: &MaterialCompositionRow) -> BpaResult<MaterialComposition> {
        let private_json = self.encryption.decrypt(&row.encrypted_details)?;
        let private: PrivateFields = serde_json::from_str(&private_json)
            .map_err(|e| BpaError::Internal(format!("deserialize private fields: {}", e)))?;

        Ok(MaterialComposition {
            bpan: row.bpan.clone(),
            cathode_material: row.cathode_material.clone(),
            anode_material: row.anode_material.clone(),
            electrolyte_type: row.electrolyte_type.clone(),
            separator_material: row.separator_material.clone(),
            recyclable_percentage: row.recyclable_percentage,
            lithium_content_g: private.lithium_content_g,
            cobalt_content_g: private.cobalt_content_g,
            nickel_content_g: private.nickel_content_g,
            manganese_content_g: private.manganese_content_g,
            lead_content_g: private.lead_content_g,
            cadmium_content_g: private.cadmium_content_g,
            hazardous_substances: private.hazardous_substances,
            supply_chain_source: private.supply_chain_source,
        })
    }

    /// Return only the public subset (no decryption needed).
    pub fn to_public(row: &MaterialCompositionRow) -> MaterialCompositionPublic {
        MaterialCompositionPublic {
            bpan: row.bpan.clone(),
            cathode_material: row.cathode_material.clone(),
            anode_material: row.anode_material.clone(),
            electrolyte_type: row.electrolyte_type.clone(),
            separator_material: row.separator_material.clone(),
            recyclable_percentage: row.recyclable_percentage,
        }
    }

    /// Check whether a role may see private material fields.
    pub fn can_see_private(role: &str) -> bool {
        PRIVATE_ROLES.iter().any(|r| r.eq_ignore_ascii_case(role))
    }

    /// SHA-256 hash of the full composition (for audit / static_data_submission_log).
    fn compute_data_hash(comp: &MaterialComposition) -> String {
        let mut hasher = Sha256::new();
        hasher.update(comp.bpan.as_bytes());
        hasher.update(comp.cathode_material.as_bytes());
        hasher.update(comp.anode_material.as_bytes());
        hasher.update(comp.electrolyte_type.as_bytes());
        hasher.update(comp.separator_material.as_bytes());
        hasher.update(comp.recyclable_percentage.to_le_bytes());
        hasher.update(comp.lithium_content_g.to_le_bytes());
        hasher.update(comp.cobalt_content_g.to_le_bytes());
        hasher.update(comp.nickel_content_g.to_le_bytes());
        hasher.update(comp.manganese_content_g.to_le_bytes());
        hasher.update(comp.lead_content_g.to_le_bytes());
        hasher.update(comp.cadmium_content_g.to_le_bytes());
        hasher.update(comp.hazardous_substances.as_bytes());
        hasher.update(comp.supply_chain_source.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// ─── Unit tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_encryption_svc() -> EncryptionService {
        // 32-char ASCII key for tests
        EncryptionService::new("01234567890123456789012345678901").unwrap()
    }

    fn sample_composition() -> MaterialComposition {
        MaterialComposition {
            bpan: "MY008A6FKKKLC1DH80001".into(),
            cathode_material: "NMC811".into(),
            anode_material: "Graphite".into(),
            electrolyte_type: "LiPF6".into(),
            separator_material: "PE/PP".into(),
            recyclable_percentage: 92.5,
            lithium_content_g: 450.0,
            cobalt_content_g: 120.0,
            nickel_content_g: 310.0,
            manganese_content_g: 85.0,
            lead_content_g: 0.0,
            cadmium_content_g: 0.0,
            hazardous_substances: "LiPF6".into(),
            supply_chain_source: "Korea/Posco".into(),
        }
    }

    #[test]
    fn test_prepare_and_decrypt_roundtrip() {
        let svc = MaterialService::new(test_encryption_svc());
        let comp = sample_composition();

        let (row, hash) = svc.prepare_submission(&comp).unwrap();

        // Public fields are plaintext
        assert_eq!(row.cathode_material, "NMC811");
        assert_eq!(row.recyclable_percentage, 92.5);

        // Encrypted details is base64 ciphertext (not plaintext JSON)
        assert!(!row.encrypted_details.contains("lithium"));
        assert!(!hash.is_empty());

        // Decrypt and verify round-trip
        let decrypted = svc.decrypt_row(&row).unwrap();
        assert_eq!(decrypted.lithium_content_g, 450.0);
        assert_eq!(decrypted.cobalt_content_g, 120.0);
        assert_eq!(decrypted.supply_chain_source, "Korea/Posco");
        assert_eq!(decrypted.bpan, comp.bpan);
    }

    #[test]
    fn test_public_view_hides_private() {
        let svc = MaterialService::new(test_encryption_svc());
        let comp = sample_composition();
        let (row, _) = svc.prepare_submission(&comp).unwrap();

        let public = MaterialService::to_public(&row);
        assert_eq!(public.cathode_material, "NMC811");
        assert_eq!(public.recyclable_percentage, 92.5);
        // No private fields on the public struct at all (compile-time guarantee)
    }

    #[test]
    fn test_role_access() {
        assert!(MaterialService::can_see_private("manufacturer"));
        assert!(MaterialService::can_see_private("MANUFACTURER"));
        assert!(MaterialService::can_see_private("admin"));
        assert!(MaterialService::can_see_private("government"));
        assert!(!MaterialService::can_see_private("consumer"));
        assert!(!MaterialService::can_see_private("public"));
        assert!(!MaterialService::can_see_private("service_provider"));
    }

    #[test]
    fn test_data_hash_deterministic() {
        let comp = sample_composition();
        let h1 = MaterialService::compute_data_hash(&comp);
        let h2 = MaterialService::compute_data_hash(&comp);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex = 64 chars
    }
}
