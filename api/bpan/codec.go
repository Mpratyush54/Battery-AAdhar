// codec.go — BPAN encoding and decoding
// Implements the 21-character Battery Pack Aadhaar Number format per spec Section 14.

package bpan

import (
	"fmt"
	"strings"
)

// ValidCharset is the allowed character set for BPAN.
// Excludes I, O to avoid confusion. (Spec example includes '0')
const ValidCharset = "ABCDEFGHJKLMNPRSTUVWXYZ0123456789"

// BPAN represents a parsed 21-character Battery Pack Aadhaar Number.
type BPAN struct {
	// BMI (positions 1–5)
	CountryCode      string // 2 chars, e.g., "MY" for India
	ManufacturerCode string // 3 chars, e.g., "008"

	// BDS (battery descriptor)
	CapacityCode    string // 1 char, e.g., "A" (lookup to kWh)
	ChemistryCode   string // 1 char, e.g., "6" (lookup to chemistry type)
	VoltageCode     string // 1 char, e.g., "F" (lookup to volts)
	CellOriginCode  string // 1 char, e.g., "K" (lookup to country)
	ExtinguisherCode string // 1 char, e.g., "K" (lookup to class)
	// 3 more BDS chars in the spec (Table 4 lists 5 BDS parameters)
	BDSExtra1       string // 1 char (reserved/placeholder)
	BDSExtra2       string // 1 char (reserved/placeholder)
	BDSExtra3       string // 1 char (reserved/placeholder)

	// BI (battery identifier)
	ManufacturingYear  string // 1 char, e.g., "1" (2025)
	ManufacturingMonth string // 1 char, e.g., "D" (April)
	ManufacturingDay   string // 1 char, e.g., "H" (17)
	FactoryCode        string // 1 char, e.g., "8"
	SequentialNumber   string // 4 chars, e.g., "0001"

	// Raw 21-char string (for reference)
	Raw string
}

// CodecError represents a BPAN codec error.
type CodecError struct {
	Code    string
	Message string
}

func (e CodecError) Error() string {
	return fmt.Sprintf("BPAN codec error (%s): %s", e.Code, e.Message)
}

// IsValidCharacter checks if a character is allowed in BPAN.
func IsValidCharacter(ch byte) bool {
	return strings.ContainsRune(ValidCharset, rune(ch))
}

// ValidateFormat checks if a 21-char string conforms to BPAN format.
func ValidateFormat(bpan string) error {
	if len(bpan) != 21 {
		return CodecError{"LENGTH", fmt.Sprintf("expected 21 chars, got %d", len(bpan))}
	}

	for i, ch := range bpan {
		if !IsValidCharacter(byte(ch)) {
			return CodecError{
				"INVALID_CHAR",
				fmt.Sprintf("position %d: '%c' not in allowed charset", i+1, ch),
			}
		}
	}

	return nil
}

// Decode parses a 21-char BPAN string into structured fields.
func Decode(bpanStr string) (*BPAN, error) {
	if err := ValidateFormat(bpanStr); err != nil {
		return nil, err
	}

	bpan := &BPAN{
		// BMI
		CountryCode:      bpanStr[0:2],
		ManufacturerCode: bpanStr[2:5],

		// BDS (positions 5–13 in the spec pilot example: A6FKKKLC)
		// But let's follow the actual positions from the spec
		// After discussion: positions 6–13 are "Battery Descriptor & Identifier combined"
		// For safety, we'll parse them in order:
		CapacityCode:     string(bpanStr[5]),  // position 6
		ChemistryCode:    string(bpanStr[6]),  // position 7
		VoltageCode:      string(bpanStr[7]),  // position 8
		CellOriginCode:   string(bpanStr[8]),  // position 9
		ExtinguisherCode: string(bpanStr[9]),  // position 10
		BDSExtra1:        string(bpanStr[10]), // position 11
		BDSExtra2:        string(bpanStr[11]), // position 12
		BDSExtra3:        string(bpanStr[12]), // position 13

		// BI (positions 14–21)
		ManufacturingYear:  string(bpanStr[13]), // position 14
		ManufacturingMonth: string(bpanStr[14]), // position 15
		ManufacturingDay:   string(bpanStr[15]), // position 16
		FactoryCode:        string(bpanStr[16]), // position 17
		SequentialNumber:   bpanStr[17:21],      // positions 18–21 (4 chars)

		Raw: bpanStr,
	}

	return bpan, nil
}

// Encode constructs a 21-char BPAN from structured fields.
func Encode(
	countryCode, manufacturerCode,
	capacityCode, chemistryCode, voltageCode, cellOriginCode, extinguisherCode,
	bdsExtra1, bdsExtra2, bdsExtra3,
	manufacturingYear, manufacturingMonth, manufacturingDay, factoryCode,
	sequentialNumber string,
) (string, error) {
	// Validate all components are present and have correct length
	parts := []struct {
		name  string
		value string
		len   int
	}{
		{"countryCode", countryCode, 2},
		{"manufacturerCode", manufacturerCode, 3},
		{"capacityCode", capacityCode, 1},
		{"chemistryCode", chemistryCode, 1},
		{"voltageCode", voltageCode, 1},
		{"cellOriginCode", cellOriginCode, 1},
		{"extinguisherCode", extinguisherCode, 1},
		{"bdsExtra1", bdsExtra1, 1},
		{"bdsExtra2", bdsExtra2, 1},
		{"bdsExtra3", bdsExtra3, 1},
		{"manufacturingYear", manufacturingYear, 1},
		{"manufacturingMonth", manufacturingMonth, 1},
		{"manufacturingDay", manufacturingDay, 1},
		{"factoryCode", factoryCode, 1},
		{"sequentialNumber", sequentialNumber, 4},
	}

	var result strings.Builder
	result.Grow(21)

	for _, part := range parts {
		if len(part.value) != part.len {
			return "", CodecError{
				"INVALID_COMPONENT",
				fmt.Sprintf("%s: expected %d chars, got %d", part.name, part.len, len(part.value)),
			}
		}
		// Validate each character
		for _, ch := range part.value {
			if !IsValidCharacter(byte(ch)) {
				return "", CodecError{
					"INVALID_CHAR",
					fmt.Sprintf("%s: '%c' not in allowed charset", part.name, ch),
				}
			}
		}
		result.WriteString(strings.ToUpper(part.value))
	}

	bpanStr := result.String()
	if err := ValidateFormat(bpanStr); err != nil {
		return "", err
	}

	return bpanStr, nil
}

// EncodeFromBPAN is a convenience method to re-encode a decoded BPAN.
func (b *BPAN) Encode() (string, error) {
	return Encode(
		b.CountryCode, b.ManufacturerCode,
		b.CapacityCode, b.ChemistryCode, b.VoltageCode, b.CellOriginCode, b.ExtinguisherCode,
		b.BDSExtra1, b.BDSExtra2, b.BDSExtra3,
		b.ManufacturingYear, b.ManufacturingMonth, b.ManufacturingDay, b.FactoryCode,
		b.SequentialNumber,
	)
}

// String returns the raw 21-char BPAN string.
func (b *BPAN) String() string {
	return b.Raw
}
