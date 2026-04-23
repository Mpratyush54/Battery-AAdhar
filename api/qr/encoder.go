// encoder.go — QR code generation
// Encodes BPAN + key battery data into QR payloads

package qr

import (
	"encoding/json"
	"fmt"

	"github.com/Mpratyush54/Battery-AAdhar/api/bpan"
	"github.com/skip2/go-qrcode"
)

// QRPayload is the data embedded in the QR code.
// Includes BPAN + essential static data for offline scanning.
type QRPayload struct {
	BPAN                  string  `json:"bpan"`
	CountryCode           string  `json:"country_code"`
	ManufacturerCode      string  `json:"manufacturer_code"`
	CapacityKwh           float32 `json:"capacity_kwh"`
	ChemistryType         string  `json:"chemistry_type"`
	NominalVoltageV       float32 `json:"nominal_voltage_v"`
	ManufacturingYear     int     `json:"manufacturing_year"`
	ManufacturingMonth    string  `json:"manufacturing_month"`
	ManufacturingDay      int     `json:"manufacturing_day"`
	RecyclePercentage     float32 `json:"recycle_percentage"`
	CarbonFootprintKgCO2e float32 `json:"carbon_footprint_kgco2e"`
}

// CreatePayload builds a QR payload from a decoded BPAN.
func CreatePayload(bpanStr string) (*QRPayload, error) {
	decoded, err := bpan.Decode(bpanStr)
	if err != nil {
		return nil, fmt.Errorf("decode BPAN: %w", err)
	}

	details := decoded.DecodeDetails()

	return &QRPayload{
		BPAN:               bpanStr,
		CountryCode:        decoded.CountryCode,
		ManufacturerCode:   decoded.ManufacturerCode,
		CapacityKwh:        details.CapacityKwh,
		ChemistryType:      details.ChemistryType,
		NominalVoltageV:    details.NominalVoltageV,
		ManufacturingYear:  details.ManufacturingYear,
		ManufacturingMonth: details.ManufacturingMonth,
		ManufacturingDay:   details.ManufacturingDay,
		// These fields would come from the database (DB lookup on Day 7)
		RecyclePercentage:     0,  // TODO: fetch from battery_material_composition
		CarbonFootprintKgCO2e: 0,  // TODO: fetch from carbon_footprint
	}, nil
}

// GenerateQR creates a QR code image (PNG) from a payload.
// Size is 256x256px (QR version auto-determined based on data size).
func GenerateQR(payload *QRPayload) ([]byte, error) {
	// Serialize payload to JSON
	jsonBytes, err := json.Marshal(payload)
	if err != nil {
		return nil, fmt.Errorf("marshal payload: %w", err)
	}

	// Generate QR code
	qr, err := qrcode.New(string(jsonBytes), qrcode.Medium)
	if err != nil {
		return nil, fmt.Errorf("qrcode.New: %w", err)
	}

	// Encode as PNG
	png, err := qr.PNG(256)
	if err != nil {
		return nil, fmt.Errorf("qr.PNG: %w", err)
	}

	return png, nil
}

// PayloadString returns the JSON string embedded in the QR code.
// Useful for offline scanning verification.
func (p *QRPayload) PayloadString() (string, error) {
	data, err := json.Marshal(p)
	if err != nil {
		return "", err
	}
	return string(data), nil
}
