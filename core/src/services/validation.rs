use crate::errors::{BpaError, BpaResult};

/// Input validation service for the BPA platform.
/// Validates all incoming data against the BPA guideline business rules before
/// it enters the database layer.
pub struct ValidationService;

impl ValidationService {
    // --- Chemistry validation ---

    /// Validate chemistry type string (human-readable form).
    pub fn validate_chemistry_type(chemistry: &str) -> BpaResult<()> {
        let valid = [
            "LFP",
            "NMC",
            "NCA",
            "LTO",
            "Solid-State",
            "NaIon",
            "Other",
            "lfp",
            "nmc",
            "nca",
            "lto",
            "solid-state",
            "naion",
            "other",
        ];
        if !valid.iter().any(|v| v.eq_ignore_ascii_case(chemistry)) {
            return Err(BpaError::Validation(format!(
                "Invalid chemistry type '{}'. Valid: LFP, NMC, NCA, LTO, Solid-State, NaIon, Other",
                chemistry
            )));
        }
        Ok(())
    }

    // --- Battery descriptor validation ---

    /// Validate nominal voltage (must be positive and within a sane range).
    pub fn validate_voltage(voltage: f64) -> BpaResult<()> {
        if voltage <= 0.0 || voltage > 1000.0 {
            return Err(BpaError::Validation(format!(
                "Nominal voltage must be between 0 and 1000V, got {:.2}",
                voltage
            )));
        }
        Ok(())
    }

    /// Validate rated capacity in kWh.
    pub fn validate_capacity(capacity_kwh: f64) -> BpaResult<()> {
        if capacity_kwh <= 0.0 || capacity_kwh > 10000.0 {
            return Err(BpaError::Validation(format!(
                "Rated capacity must be between 0 and 10000 kWh, got {:.2}",
                capacity_kwh
            )));
        }
        Ok(())
    }

    /// Validate energy density (Wh/kg).
    pub fn validate_energy_density(density: f64) -> BpaResult<()> {
        if density <= 0.0 || density > 2000.0 {
            return Err(BpaError::Validation(format!(
                "Energy density must be between 0 and 2000 Wh/kg, got {:.2}",
                density
            )));
        }
        Ok(())
    }

    /// Validate weight in kg.
    pub fn validate_weight(weight_kg: f64) -> BpaResult<()> {
        if weight_kg <= 0.0 || weight_kg > 50000.0 {
            return Err(BpaError::Validation(format!(
                "Weight must be between 0 and 50000 kg, got {:.2}",
                weight_kg
            )));
        }
        Ok(())
    }

    /// Validate form factor.
    pub fn validate_form_factor(form_factor: &str) -> BpaResult<()> {
        let valid = [
            "cylindrical",
            "prismatic",
            "pouch",
            "blade",
            "module",
            "other",
        ];
        if !valid.iter().any(|v| v.eq_ignore_ascii_case(form_factor)) {
            return Err(BpaError::Validation(format!(
                "Invalid form factor '{}'. Valid: cylindrical, prismatic, pouch, blade, module, other",
                form_factor
            )));
        }
        Ok(())
    }

    // --- Material composition validation ---

    /// Validate material content in grams (must be non-negative).
    pub fn validate_material_content(name: &str, grams: f64) -> BpaResult<()> {
        if grams < 0.0 {
            return Err(BpaError::Validation(format!(
                "{} content cannot be negative, got {:.2}g",
                name, grams
            )));
        }
        Ok(())
    }

    /// Validate recyclable percentage (0-100).
    pub fn validate_percentage(name: &str, value: f64) -> BpaResult<()> {
        if !(0.0..=100.0).contains(&value) {
            return Err(BpaError::Validation(format!(
                "{} must be between 0 and 100%, got {:.2}",
                name, value
            )));
        }
        Ok(())
    }

    // --- State of Health validation ---

    /// Validate SoH value (0-100%).
    pub fn validate_soh(soh: f64) -> BpaResult<()> {
        Self::validate_percentage("State of Health", soh)
    }

    /// Validate total charge/discharge cycles.
    pub fn validate_cycle_count(cycles: i32) -> BpaResult<()> {
        if cycles < 0 {
            return Err(BpaError::Validation(format!(
                "Cycle count cannot be negative, got {}",
                cycles
            )));
        }
        Ok(())
    }

    // --- Carbon footprint validation ---

    /// Validate an emission value (must be non-negative, in kg CO2e).
    pub fn validate_emission(stage: &str, value: f64) -> BpaResult<()> {
        if value < 0.0 {
            return Err(BpaError::Validation(format!(
                "{} emission cannot be negative, got {:.2} kg CO2e",
                stage, value
            )));
        }
        Ok(())
    }

    // --- Battery category validation ---

    /// Validate battery category (per BPA scope: EV L/M/N categories, Industrial >2kWh).
    pub fn validate_battery_category(category: &str) -> BpaResult<()> {
        let valid = ["EV-L", "EV-M", "EV-N", "Industrial", "ESS"];
        if !valid.iter().any(|v| v.eq_ignore_ascii_case(category)) {
            return Err(BpaError::Validation(format!(
                "Invalid battery category '{}'. Valid: EV-L, EV-M, EV-N, Industrial, ESS",
                category
            )));
        }
        Ok(())
    }

    /// Validate compliance class.
    pub fn validate_compliance_class(class: &str) -> BpaResult<()> {
        let valid = [
            "AIS-156",
            "AIS-038",
            "IS-16893",
            "IEC-62660",
            "UN-38.3",
            "OTHER",
        ];
        if !valid.iter().any(|v| v.eq_ignore_ascii_case(class)) {
            return Err(BpaError::Validation(format!(
                "Invalid compliance class '{}'. Valid: AIS-156, AIS-038, IS-16893, IEC-62660, UN-38.3, OTHER",
                class
            )));
        }
        Ok(())
    }

    // --- Stakeholder validations ---

    /// Validate stakeholder role.
    pub fn validate_stakeholder_role(role: &str) -> BpaResult<()> {
        let valid = [
            "MANUFACTURER",
            "IMPORTER",
            "OEM",
            "SERVICE_PROVIDER",
            "RECYCLER",
            "REGULATOR",
            "CONSUMER",
            "AUDITOR",
            "SYSTEM_ADMIN",
        ];
        if !valid.iter().any(|v| v.eq_ignore_ascii_case(role)) {
            return Err(BpaError::Validation(format!(
                "Invalid stakeholder role '{}'",
                role
            )));
        }
        Ok(())
    }

    /// Validate access level.
    pub fn validate_access_level(level: &str) -> BpaResult<()> {
        let valid = ["READ", "WRITE", "ADMIN", "REGULATOR_READ", "AUDIT_READ"];
        if !valid.iter().any(|v| v.eq_ignore_ascii_case(level)) {
            return Err(BpaError::Validation(format!(
                "Invalid access level '{}'",
                level
            )));
        }
        Ok(())
    }

    // --- Generic string validation ---

    /// Validate a non-empty string field.
    pub fn validate_non_empty(field_name: &str, value: &str) -> BpaResult<()> {
        if value.trim().is_empty() {
            return Err(BpaError::Validation(format!(
                "'{}' cannot be empty",
                field_name
            )));
        }
        Ok(())
    }

    /// Validate a country code (2-letter ISO 3166-1 alpha-2).
    pub fn validate_country_code(code: &str) -> BpaResult<()> {
        if code.len() != 2 || !code.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(BpaError::Validation(format!(
                "Country code must be 2 uppercase letters (ISO 3166-1 alpha-2), got '{}'",
                code
            )));
        }
        Ok(())
    }

    /// Validate degradation class.
    pub fn validate_degradation_class(class: &str) -> BpaResult<()> {
        let valid = ["A", "B", "C", "D", "F"];
        if !valid.iter().any(|v| v.eq_ignore_ascii_case(class)) {
            return Err(BpaError::Validation(format!(
                "Invalid degradation class '{}'. Valid: A (best), B, C, D, F (worst)",
                class
            )));
        }
        Ok(())
    }
}
