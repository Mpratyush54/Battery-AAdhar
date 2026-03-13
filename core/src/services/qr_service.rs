use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::errors::{BpaError, BpaResult};
use crate::services::hash_chain::HashChainService;

/// QR code data payload for Layer 2 of the BPA system.
/// The QR code encodes static battery attributes for offline accessibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrPayload {
    /// The 21-character Battery Pack Aadhaar Number
    pub bpan: String,
    /// Chemistry type (e.g., LFP, NMC)
    pub chemistry_type: String,
    /// Nominal voltage in Volts
    pub nominal_voltage: f64,
    /// Rated capacity in kWh
    pub rated_capacity_kwh: f64,
    /// Energy density in Wh/kg
    pub energy_density: f64,
    /// Weight in kg
    pub weight_kg: f64,
    /// Form factor (cylindrical, prismatic, pouch)
    pub form_factor: String,
    /// Manufacturer name (from manufacturer record)
    pub manufacturer_name: String,
    /// Production year
    pub production_year: i32,
    /// Cathode material composition
    pub cathode_material: String,
    /// Anode material composition
    pub anode_material: String,
    /// Electrolyte type
    pub electrolyte_type: String,
    /// Recyclable percentage
    pub recyclable_percentage: f64,
    /// Total carbon footprint (kg CO2e) — Phase 3
    pub carbon_footprint_total: Option<f64>,
    /// SHA-256 hash of the static data for integrity verification
    pub data_hash: String,
}

pub struct QrService;

impl QrService {
    /// Build a QR payload from the battery's static data.
    /// The `data_hash` is computed over the critical fields so the QR can be
    /// verified against the server-side record without connectivity.
    #[instrument(name = "build_qr_payload", skip_all)]
    pub fn build_payload(
        bpan: &str,
        chemistry_type: &str,
        nominal_voltage: f64,
        rated_capacity_kwh: f64,
        energy_density: f64,
        weight_kg: f64,
        form_factor: &str,
        manufacturer_name: &str,
        production_year: i32,
        cathode_material: &str,
        anode_material: &str,
        electrolyte_type: &str,
        recyclable_percentage: f64,
        carbon_footprint_total: Option<f64>,
    ) -> BpaResult<QrPayload> {
        // Compute the data hash
        let data_hash = HashChainService::compute_static_hash(
            bpan,
            chemistry_type,
            nominal_voltage,
            rated_capacity_kwh,
            form_factor,
        );

        let payload = QrPayload {
            bpan: bpan.to_string(),
            chemistry_type: chemistry_type.to_string(),
            nominal_voltage,
            rated_capacity_kwh,
            energy_density,
            weight_kg,
            form_factor: form_factor.to_string(),
            manufacturer_name: manufacturer_name.to_string(),
            production_year,
            cathode_material: cathode_material.to_string(),
            anode_material: anode_material.to_string(),
            electrolyte_type: electrolyte_type.to_string(),
            recyclable_percentage,
            carbon_footprint_total,
            data_hash,
        };

        info!("Built QR payload for BPAN: {}", bpan);
        Ok(payload)
    }

    /// Serialize the QR payload to a compact JSON string suitable for QR encoding.
    pub fn encode_payload(payload: &QrPayload) -> BpaResult<String> {
        serde_json::to_string(payload).map_err(|e| {
            BpaError::QrError(format!("Failed to serialize QR payload: {}", e))
        })
    }

    /// Deserialize a QR payload from a JSON string.
    pub fn decode_payload(json_str: &str) -> BpaResult<QrPayload> {
        serde_json::from_str(json_str).map_err(|e| {
            BpaError::QrError(format!("Failed to deserialize QR payload: {}", e))
        })
    }

    /// Verify the integrity of a QR payload by recomputing the data hash.
    pub fn verify_payload(payload: &QrPayload) -> BpaResult<()> {
        let computed_hash = HashChainService::compute_static_hash(
            &payload.bpan,
            &payload.chemistry_type,
            payload.nominal_voltage,
            payload.rated_capacity_kwh,
            &payload.form_factor,
        );

        if computed_hash != payload.data_hash {
            return Err(BpaError::IntegrityViolation(format!(
                "QR payload hash mismatch for BPAN {}: expected {}, computed {}",
                payload.bpan, payload.data_hash, computed_hash
            )));
        }

        info!("QR payload integrity verified for BPAN: {}", payload.bpan);
        Ok(())
    }

    /// Compute the hash of the entire QR payload (for storage in qr_records).
    pub fn compute_payload_hash(payload: &QrPayload) -> BpaResult<String> {
        let json = Self::encode_payload(payload)?;
        Ok(HashChainService::compute_hash(&json))
    }
}
