// lookup.go — BPAN lookup tables (spec Annexures I–IV)
// Maps character codes to actual values (country names, chemistry types, etc.)

package bpan

var (
	// Table 16: Country codes (partial — India focus)
	CountryCodeMap = map[string]string{
		"MA": "Algeria",
		"MY": "India",
		"KA": "Sri Lanka",
		"KL": "Korea",
		"SA": "United Kingdom",
		"SN": "Germany",
		"UA": "Romania",
		"VA": "Austria",
		"VF": "France",
		"VS": "Spain",
		"XA": "Bulgaria",
		"YA": "Belgium",
		"ZA": "Italy",
	}

	// Table 18: Battery chemistry
	ChemistryMap = map[string]string{
		"A": "Lead Acid",
		"B": "Nickel-Cadmium",
		"C": "Nickel-Metal Hydride",
		"D": "Sodium-Ion",
		"E": "LFP",
		"F": "NMC",
		"G": "Reserved",
		"H": "Reserved",
		"J": "Reserved",
		"K": "Reserved",
		"L": "Reserved",
	}

	// Table 19: Extinguishing agent / class
	ExtinguisherMap = map[string]string{
		"A": "Class A",
		"B": "Class B",
		"C": "Class C",
		"D": "Class D",
		"E": "Class K",
	}

	// Table 20: Manufacturing year (base 2025)
	ManufacturingYearMap = map[string]int{
		"1": 2025, "2": 2026, "3": 2027, "4": 2028, "5": 2029,
		"6": 2030, "7": 2031, "8": 2032, "9": 2033,
		"A": 2034, "B": 2035, "C": 2036, "D": 2037, "E": 2038,
		"F": 2039, "G": 2040, "H": 2041, "J": 2042, "K": 2043,
		"L": 2044, "M": 2045, "N": 2046, "P": 2047, "Q": 2048,
		"R": 2049, "S": 2050, "T": 2051, "U": 2052, "V": 2053,
		"W": 2054, "X": 2055, "Y": 2056, "Z": 2057,
	}

	// Table 21: Manufacturing month
	ManufacturingMonthMap = map[string]string{
		"A": "January",
		"B": "February",
		"C": "March",
		"D": "April",
		"E": "May",
		"F": "June",
		"G": "July",
		"H": "August",
		"J": "September",
		"K": "October",
		"L": "November",
		"M": "December",
	}

	// Table 22: Manufacturing date (day of month)
	ManufacturingDateMap = map[string]int{
		"1": 1, "2": 2, "3": 3, "4": 4, "5": 5,
		"6": 6, "7": 7, "8": 8, "9": 9,
		"A": 10, "B": 11, "C": 12, "D": 13, "E": 14,
		"F": 15, "G": 16, "H": 17, "J": 18, "K": 19,
		"L": 20, "M": 21, "N": 22, "P": 23, "Q": 24,
		"R": 25, "S": 26, "T": 27, "U": 28, "V": 29,
		"W": 30, "X": 31,
	}

	// Table 23: Factory code
	FactoryCodeMap = map[string]int{
		"1": 1, "2": 2, "3": 3, "4": 4, "5": 5,
		"6": 6, "7": 7, "8": 8, "9": 9,
		"A": 10, "B": 11, "C": 12, "D": 13, "E": 14,
		"F": 15, "G": 16, "H": 17, "J": 18, "K": 19,
		"L": 20, "M": 21, "N": 22, "P": 23, "Q": 24,
		"R": 25, "S": 26, "T": 27, "U": 28, "V": 29,
		"W": 30, "X": 31, "Y": 32, "Z": 33,
	}

	// Capacity lookup (simplified from Annexure IV)
	// Maps single character to kWh value
	CapacityMap = map[string]float32{
		"A": 30, // AA in Annexure IV = 1 → mapped here as convenience
		"B": 50,
		"C": 75,
		"D": 100,
		"E": 150,
		"F": 200,
		// ... more as needed
	}

	// Voltage lookup (simplified)
	VoltageMap = map[string]float32{
		"F": 307, // Example from spec pilot
		"K": 400,
		"L": 450,
		// ... more as needed
	}
)

// DecodedBPANDetails returns human-readable details of a decoded BPAN.
type DecodedBPANDetails struct {
	CountryName        string
	ManufacturerCode   string
	CapacityKwh        float32
	ChemistryType      string
	NominalVoltageV    float32
	CellOrigin         string
	ExtinguisherClass  string
	ManufacturingYear  int
	ManufacturingMonth string
	ManufacturingDay   int
	FactoryNumber      int
	SequentialNumber   string
}

// Decode a BPAN and return human-readable fields.
func (b *BPAN) DecodeDetails() *DecodedBPANDetails {
	return &DecodedBPANDetails{
		CountryName:        CountryCodeMap[b.CountryCode],
		ManufacturerCode:   b.ManufacturerCode,
		CapacityKwh:        CapacityMap[b.CapacityCode],
		ChemistryType:      ChemistryMap[b.ChemistryCode],
		NominalVoltageV:    VoltageMap[b.VoltageCode],
		CellOrigin:         CountryCodeMap[b.CellOriginCode],
		ExtinguisherClass:  ExtinguisherMap[b.ExtinguisherCode],
		ManufacturingYear:  ManufacturingYearMap[b.ManufacturingYear],
		ManufacturingMonth: ManufacturingMonthMap[b.ManufacturingMonth],
		ManufacturingDay:   ManufacturingDateMap[b.ManufacturingDay],
		FactoryNumber:      FactoryCodeMap[b.FactoryCode],
		SequentialNumber:   b.SequentialNumber,
	}
}
