// bpan_test.go — BPAN codec tests
// Test case from spec Section 15.1 (pilot example)

package bpan

import (
	"testing"
)

func TestDecodeSpecPilot(t *testing.T) {
	// Spec Section 15.1 pilot example
	pilotBPAN := "MY008A6FKKKLC1DH80001"

	bpan, err := Decode(pilotBPAN)
	if err != nil {
		t.Fatalf("Decode failed: %v", err)
	}

	// Verify each component
	tests := []struct {
		field    string
		expected string
		actual   string
	}{
		{"CountryCode", "MY", bpan.CountryCode},
		{"ManufacturerCode", "008", bpan.ManufacturerCode},
		{"CapacityCode", "A", bpan.CapacityCode},
		{"ChemistryCode", "6", bpan.ChemistryCode},
		{"VoltageCode", "F", bpan.VoltageCode},
		{"CellOriginCode", "K", bpan.CellOriginCode},
		{"ExtinguisherCode", "K", bpan.ExtinguisherCode},
		{"ManufacturingYear", "1", bpan.ManufacturingYear},
		{"ManufacturingMonth", "D", bpan.ManufacturingMonth},
		{"ManufacturingDay", "H", bpan.ManufacturingDay},
		{"FactoryCode", "8", bpan.FactoryCode},
		{"SequentialNumber", "0001", bpan.SequentialNumber},
	}

	for _, tt := range tests {
		if tt.actual != tt.expected {
			t.Errorf("%s: expected %q, got %q", tt.field, tt.expected, tt.actual)
		}
	}
}

func TestEncodeSpecPilot(t *testing.T) {
	// Reverse of Decode test: encode the same components
	encoded, err := Encode(
		"MY", "008",           
		"A", "6", "F", "K", "K",  
		"K", "L", "C",               
		"1", "D", "H", "8", "0001", 
	)
	
	if err != nil {
		t.Fatalf("Encode failed: %v", err)
	}

	expected := "MY008A6FKKKLC1DH80001"
	if len(encoded) != 21 {
		t.Errorf("Encoded BPAN length: expected 21, got %d", len(encoded))
	}

	if encoded != expected {
		t.Errorf("Encode mismatch: expected %s, got %s", expected, encoded)
	}
}

func TestValidateFormat(t *testing.T) {
	tests := []struct {
		name      string
		bpan      string
		shouldErr bool
	}{
		{"Valid pilot", "MY008A6FKKKLC1DH80001", false},
		{"Too short", "MY008A6FKKK", true},
		{"Too long", "MY008A6FKKKLC1DH800011", true},
		{"Invalid char (I)", "MY008I6FKKKLC1DH80001", true},
		{"Invalid char (O)", "MY008O6FKKKLC1DH80001", true},
		{"Invalid char (0)", "MY008061FKKKLC1DH80001", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateFormat(tt.bpan)
			if (err != nil) != tt.shouldErr {
				t.Errorf("ValidateFormat: shouldErr=%v, got err=%v", tt.shouldErr, err)
			}
		})
	}
}

func TestDecodeEncode(t *testing.T) {
	// Decode then re-encode should produce the same BPAN
	original := "MY008A6FKKKLC1DH80001"

	decoded, err := Decode(original)
	if err != nil {
		t.Fatalf("Decode failed: %v", err)
	}

	reencoded, err := decoded.Encode()
	if err != nil {
		t.Fatalf("Re-encode failed: %v", err)
	}

	if reencoded != original {
		t.Errorf("Round-trip failed: original=%s, reencoded=%s", original, reencoded)
	}
}

func TestIsValidCharacter(t *testing.T) {
	validChars := []string{"A", "1", "Z", "9"}
	invalidChars := []string{"I", "O", "@", " "}

	for _, ch := range validChars {
		if !IsValidCharacter(ch[0]) {
			t.Errorf("IsValidCharacter(%s): should be true", ch)
		}
	}

	for _, ch := range invalidChars {
		if IsValidCharacter(ch[0]) {
			t.Errorf("IsValidCharacter(%s): should be false", ch)
		}
	}
}
