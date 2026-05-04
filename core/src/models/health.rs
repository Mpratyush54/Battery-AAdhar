//! health.rs — Battery State of Health (SoH) and dynamic data models
//!
//! Represents Table 6 (BDD — Battery Dynamic Data) from spec.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Health status thresholds (SoH %)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Operational, // SoH > 80%
    SecondLife,  // 60% <= SoH <= 80%
    EolProcess,  // 30% <= SoH < 60%
    Waste,       // SoH < 30%
    Unknown,     // Not yet classified
}

impl HealthStatus {
    pub fn from_soh(soh: f32) -> Self {
        match soh {
            s if s > 80.0 => HealthStatus::Operational,
            s if (60.0..=80.0).contains(&s) => HealthStatus::SecondLife,
            s if (30.0..60.0).contains(&s) => HealthStatus::EolProcess,
            s if s < 30.0 => HealthStatus::Waste,
            _ => HealthStatus::Unknown,
        }
    }

}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Operational => write!(f, "OPERATIONAL"),
            HealthStatus::SecondLife => write!(f, "SECOND_LIFE"),
            HealthStatus::EolProcess => write!(f, "EOL_PROCESS"),
            HealthStatus::Waste => write!(f, "WASTE"),
            HealthStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Single health record (point-in-time snapshot)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthRecord {
    pub id: Uuid,
    pub bpan: String,

    // === Core SoH Data ===
    pub state_of_health_percent: f32, // 0–100%
    pub health_status: HealthStatus,

    // === Cycle & Degradation ===
    pub cycle_count: u32, // Total full-equivalent cycles completed
    pub degradation_rate_percent_per_cycle: f32, // SoH loss per cycle (~0.05–0.2%)
    pub degradation_class: String, // "fast", "normal", "slow"

    // === Temperature ===
    pub min_temperature_celsius: f32,     // Minimum experienced
    pub max_temperature_celsius: f32,     // Maximum experienced
    pub average_temperature_celsius: f32, // Average during operation

    // === Electrical ===
    pub cell_voltage_min_mv: f32,      // Minimum cell voltage observed
    pub cell_voltage_max_mv: f32,      // Maximum cell voltage observed
    pub internal_resistance_mohm: f32, // Impedance increase (indicator of degradation)

    // === Flags ===
    pub error_flags: String, // Comma-separated: "OVER_TEMP", "OVER_CURRENT", "SHORT_CIRCUIT", etc.
    pub is_healthy: bool,    // No critical errors

    // === Metadata ===
    pub reported_by: String, // BMS ID or manufacturer
    pub reported_at: DateTime<Utc>,
    pub record_number: u32, // Sequential record count per BPAN

    // === ZK Proof (auto-generated) ===
    pub zk_proof_soh_gt_80: Option<Vec<u8>>, // Proof that SoH > 80% (commitment hidden)
    pub zk_proof_soh_gte_60: Option<Vec<u8>>, // Proof that SoH >= 60%
    pub zk_proof_soh_gte_30: Option<Vec<u8>>, // Proof that SoH >= 30%
    pub proofs_generated_at: Option<DateTime<Utc>>,
}

impl HealthRecord {
    pub fn new(
        bpan: String,
        soh: f32,
        cycles: u32,
        degradation_class: String,
        reported_by: String,
    ) -> Self {
        let health_status = HealthStatus::from_soh(soh);

        HealthRecord {
            id: Uuid::new_v4(),
            bpan,
            state_of_health_percent: soh,
            health_status,
            cycle_count: cycles,
            degradation_rate_percent_per_cycle: 0.1, // Default ~0.1%/cycle for NMC
            degradation_class,
            min_temperature_celsius: 20.0,
            max_temperature_celsius: 40.0,
            average_temperature_celsius: 30.0,
            cell_voltage_min_mv: 2500.0,
            cell_voltage_max_mv: 4200.0,
            internal_resistance_mohm: 15.0,
            error_flags: String::new(),
            is_healthy: true,
            reported_by,
            reported_at: Utc::now(),
            record_number: 1,
            zk_proof_soh_gt_80: None,
            zk_proof_soh_gte_60: None,
            zk_proof_soh_gte_30: None,
            proofs_generated_at: None,
        }
    }

    /// Check if battery meets operational threshold
    pub fn is_operational(&self) -> bool {
        self.state_of_health_percent > 80.0
    }

    /// Check if battery is suitable for second-life (stationary)
    pub fn is_second_life_eligible(&self) -> bool {
        self.state_of_health_percent >= 60.0 && self.state_of_health_percent <= 80.0
    }

    /// Check if battery should be recycled
    pub fn should_recycle(&self) -> bool {
        self.state_of_health_percent < 30.0
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Request payload for health update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthUpdateRequest {
    pub state_of_health_percent: f32,
    pub cycle_count: u32,
    pub degradation_class: String,
    pub min_temperature_celsius: Option<f32>,
    pub max_temperature_celsius: Option<f32>,
    pub average_temperature_celsius: Option<f32>,
    pub cell_voltage_min_mv: Option<f32>,
    pub cell_voltage_max_mv: Option<f32>,
    pub internal_resistance_mohm: Option<f32>,
    pub error_flags: Option<String>,
}

/// Time-series health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthHistory {
    pub bpan: String,
    pub records: Vec<HealthRecord>,
    pub current_status: HealthStatus,
    pub degradation_trend: String, // "stable", "declining", "accelerating"
}

/// Health aggregate (for dashboard)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAggregate {
    pub metric: String, // "avg_soh", "median_soh", "min_soh", "max_soh"
    pub value: f32,
    pub group_by: String, // "manufacturer", "chemistry", "age_months"
    pub group_value: String,
    pub sample_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_classification() {
        assert_eq!(HealthStatus::from_soh(90.0), HealthStatus::Operational);
        assert_eq!(HealthStatus::from_soh(75.0), HealthStatus::SecondLife);
        assert_eq!(HealthStatus::from_soh(50.0), HealthStatus::EolProcess);
        assert_eq!(HealthStatus::from_soh(20.0), HealthStatus::Waste);
    }

    #[test]
    fn test_health_record_lifecycle() {
        let record = HealthRecord::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            87.5,
            150000,
            "normal".to_string(),
            "bms-001".to_string(),
        );

        assert!(record.is_operational());
        assert!(!record.is_second_life_eligible());
        assert!(!record.should_recycle());
    }

    #[test]
    fn test_health_record_serialization() {
        let record = HealthRecord::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            75.0,
            200000,
            "slow".to_string(),
            "bms-001".to_string(),
        );

        let bytes = record.to_bytes().expect("serialize failed");
        let recovered = HealthRecord::from_bytes(&bytes).expect("deserialize failed");

        assert_eq!(record.bpan, recovered.bpan);
        assert_eq!(
            record.state_of_health_percent,
            recovered.state_of_health_percent
        );
    }
}
