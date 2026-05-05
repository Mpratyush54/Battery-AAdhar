use tracing::{debug, info, instrument};

use crate::errors::{BpaError, BpaResult};

/// BPAN Format (21 characters total):
///
/// ```text
/// [BMI-3][BDS-6][BI-12]
///   │       │      └── Battery Identifier
///   │       │           ├── Serial Number (8 alphanumeric)
///   │       │           ├── Production Year (2 digits, last 2 of year)
///   │       │           └── Sequence Number (2 alphanumeric)
///   │       └── Battery Descriptor Section
///   │             ├── Chemistry Code (2 chars: LF=LFP, NM=NMC, NC=NCA, LT=LTO, SS=Solid-State, NA=NaIon, OT=Other)
///   │             ├── Category Code (2 chars: EL=EV-L, EM=EV-M, EN=EV-N, IN=Industrial, ES=ESS)
///   │             └── Capacity Band (2 chars: 01=<2kWh, 02=2-5, 03=5-10, 04=10-20, 05=20-50, 06=50-100, 07=>100)
///   └── Battery Manufacturer Identifier (3 uppercase alpha assigned by regulator)
/// ```
///
/// Example: `TAT` + `NMEL05` + `AB12345626A1` = `TATNMEL05AB12345626A1`
pub struct BpanGenerator;

/// Valid chemistry codes per BPA guideline
const VALID_CHEMISTRY_CODES: &[&str] = &["LF", "NM", "NC", "LT", "SS", "NA", "OT"];

/// Valid vehicle/battery category codes
const VALID_CATEGORY_CODES: &[&str] = &["EL", "EM", "EN", "IN", "ES"];

/// Capacity band upper bounds (in kWh) mapped to codes 01..07
const CAPACITY_BANDS: &[(f64, &str)] = &[
    (2.0, "01"),
    (5.0, "02"),
    (10.0, "03"),
    (20.0, "04"),
    (50.0, "05"),
    (100.0, "06"),
    (f64::MAX, "07"),
];

impl BpanGenerator {
    /// Generate a compliant 21-character BPAN.
    ///
    /// # Arguments
    /// * `manufacturer_code` - 3-char uppercase alpha (assigned by CPCB/regulator)
    /// * `chemistry_code` - 2-char chemistry type (e.g., "NM" for NMC)
    /// * `category_code` - 2-char vehicle/battery category (e.g., "EL" for EV L-category)
    /// * `capacity_kwh` - Rated capacity in kWh (used to derive capacity band)
    /// * `serial_number` - 8-char alphanumeric serial from manufacturer
    /// * `production_year` - Full 4-digit year (e.g., 2026)
    /// * `sequence_number` - 2-char alphanumeric sequence within the batch
    #[instrument(name = "generate_bpan", skip_all)]
    pub fn generate(
        manufacturer_code: &str,
        chemistry_code: &str,
        category_code: &str,
        capacity_kwh: f64,
        serial_number: &str,
        production_year: u16,
        sequence_number: &str,
    ) -> BpaResult<String> {
        // Validate BMI (Battery Manufacturer Identifier)
        Self::validate_manufacturer_code(manufacturer_code)?;

        // Validate chemistry code
        Self::validate_chemistry_code(chemistry_code)?;

        // Validate category code
        Self::validate_category_code(category_code)?;

        // Derive capacity band
        let capacity_band = Self::derive_capacity_band(capacity_kwh)?;

        // Validate serial number
        Self::validate_serial_number(serial_number)?;

        // Validate production year
        if !(2020..=2099).contains(&production_year) {
            return Err(BpaError::BpanFormat(
                "Production year must be between 2020 and 2099".into(),
            ));
        }
        let year_code = format!("{:02}", production_year % 100);

        // Validate sequence number
        Self::validate_sequence_number(sequence_number)?;

        // Assemble the BPAN
        let bpan = format!(
            "{}{}{}{}{}{}{}",
            manufacturer_code.to_uppercase(),
            chemistry_code.to_uppercase(),
            category_code.to_uppercase(),
            capacity_band,
            serial_number.to_uppercase(),
            year_code,
            sequence_number.to_uppercase(),
        );

        // Final length check
        if bpan.len() != 21 {
            return Err(BpaError::BpanFormat(format!(
                "Generated BPAN is {} characters, expected 21",
                bpan.len()
            )));
        }

        info!("Generated BPAN: {}", bpan);
        Ok(bpan)
    }

    /// Validate a BPAN string and return its decoded components.
    #[instrument(name = "validate_bpan")]
    pub fn validate(bpan: &str) -> BpaResult<BpanComponents> {
        if bpan.len() != 21 {
            return Err(BpaError::BpanFormat(format!(
                "BPAN must be 21 characters, got {}",
                bpan.len()
            )));
        }

        if !bpan.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(BpaError::BpanFormat(
                "BPAN must contain only ASCII alphanumeric characters".into(),
            ));
        }

        let manufacturer_code = &bpan[0..3];
        let chemistry_code = &bpan[3..5];
        let category_code = &bpan[5..7];
        let capacity_band = &bpan[7..9];
        let serial_number = &bpan[9..17];
        let year_code = &bpan[17..19];
        let sequence_number = &bpan[19..21];

        Self::validate_manufacturer_code(manufacturer_code)?;
        Self::validate_chemistry_code(chemistry_code)?;
        Self::validate_category_code(category_code)?;

        if !capacity_band.chars().all(|c| c.is_ascii_digit()) {
            return Err(BpaError::BpanFormat("Invalid capacity band".into()));
        }

        let year: u16 = year_code
            .parse::<u16>()
            .map_err(|_| BpaError::BpanFormat("Invalid year code".into()))?;
        let production_year = 2000 + year;

        debug!("BPAN {} validated successfully", bpan);

        Ok(BpanComponents {
            manufacturer_code: manufacturer_code.to_string(),
            chemistry_code: chemistry_code.to_string(),
            category_code: category_code.to_string(),
            capacity_band: capacity_band.to_string(),
            serial_number: serial_number.to_string(),
            production_year,
            sequence_number: sequence_number.to_string(),
        })
    }

    /// Decode a BPAN into human-readable component descriptions.
    pub fn decode(bpan: &str) -> BpaResult<BpanDecoded> {
        let components = Self::validate(bpan)?;

        let chemistry_name = match components.chemistry_code.as_str() {
            "LF" => "Lithium Iron Phosphate (LFP)",
            "NM" => "Nickel Manganese Cobalt (NMC)",
            "NC" => "Nickel Cobalt Aluminium (NCA)",
            "LT" => "Lithium Titanate Oxide (LTO)",
            "SS" => "Solid-State",
            "NA" => "Sodium-Ion",
            "OT" => "Other",
            _ => "Unknown",
        };

        let category_name = match components.category_code.as_str() {
            "EL" => "Electric Vehicle – L Category (2W/3W)",
            "EM" => "Electric Vehicle – M Category (Passenger)",
            "EN" => "Electric Vehicle – N Category (Commercial)",
            "IN" => "Industrial Battery (>2 kWh)",
            "ES" => "Energy Storage System",
            _ => "Unknown",
        };

        let capacity_range = match components.capacity_band.as_str() {
            "01" => "< 2 kWh",
            "02" => "2 – 5 kWh",
            "03" => "5 – 10 kWh",
            "04" => "10 – 20 kWh",
            "05" => "20 – 50 kWh",
            "06" => "50 – 100 kWh",
            "07" => "> 100 kWh",
            _ => "Unknown",
        };

        Ok(BpanDecoded {
            bpan: bpan.to_string(),
            manufacturer_code: components.manufacturer_code,
            chemistry: chemistry_name.to_string(),
            category: category_name.to_string(),
            capacity_range: capacity_range.to_string(),
            serial_number: components.serial_number,
            production_year: components.production_year,
            sequence_number: components.sequence_number,
        })
    }

    // --- Private validation helpers ---

    fn validate_manufacturer_code(code: &str) -> BpaResult<()> {
        if code.len() != 3 {
            return Err(BpaError::BpanFormat(
                "Manufacturer code must be exactly 3 characters".into(),
            ));
        }
        if !code.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(BpaError::BpanFormat(
                "Manufacturer code must be uppercase alphabetic".into(),
            ));
        }
        Ok(())
    }

    fn validate_chemistry_code(code: &str) -> BpaResult<()> {
        let upper = code.to_uppercase();
        if !VALID_CHEMISTRY_CODES.contains(&upper.as_str()) {
            return Err(BpaError::BpanFormat(format!(
                "Invalid chemistry code '{}'. Valid: {:?}",
                code, VALID_CHEMISTRY_CODES
            )));
        }
        Ok(())
    }

    fn validate_category_code(code: &str) -> BpaResult<()> {
        let upper = code.to_uppercase();
        if !VALID_CATEGORY_CODES.contains(&upper.as_str()) {
            return Err(BpaError::BpanFormat(format!(
                "Invalid category code '{}'. Valid: {:?}",
                code, VALID_CATEGORY_CODES
            )));
        }
        Ok(())
    }

    fn derive_capacity_band(capacity_kwh: f64) -> BpaResult<String> {
        if capacity_kwh <= 0.0 {
            return Err(BpaError::BpanFormat(
                "Capacity must be a positive number".into(),
            ));
        }
        for (upper_bound, code) in CAPACITY_BANDS {
            if capacity_kwh <= *upper_bound {
                return Ok(code.to_string());
            }
        }
        // Should never reach here because f64::MAX is the last bound
        Ok("07".to_string())
    }

    fn validate_serial_number(serial: &str) -> BpaResult<()> {
        if serial.len() != 8 {
            return Err(BpaError::BpanFormat(
                "Serial number must be exactly 8 alphanumeric characters".into(),
            ));
        }
        if !serial.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(BpaError::BpanFormat(
                "Serial number must be alphanumeric".into(),
            ));
        }
        Ok(())
    }

    fn validate_sequence_number(seq: &str) -> BpaResult<()> {
        if seq.len() != 2 {
            return Err(BpaError::BpanFormat(
                "Sequence number must be exactly 2 alphanumeric characters".into(),
            ));
        }
        if !seq.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(BpaError::BpanFormat(
                "Sequence number must be alphanumeric".into(),
            ));
        }
        Ok(())
    }
}

/// Parsed components of a validated BPAN.
#[derive(Debug, Clone)]
pub struct BpanComponents {
    pub manufacturer_code: String,
    pub chemistry_code: String,
    pub category_code: String,
    pub capacity_band: String,
    pub serial_number: String,
    pub production_year: u16,
    pub sequence_number: String,
}

/// Human-readable decoded BPAN.
#[derive(Debug, Clone)]
pub struct BpanDecoded {
    pub bpan: String,
    pub manufacturer_code: String,
    pub chemistry: String,
    pub category: String,
    pub capacity_range: String,
    pub serial_number: String,
    pub production_year: u16,
    pub sequence_number: String,
}
