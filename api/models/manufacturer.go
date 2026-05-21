package models

import "time"

// RegisterManufacturerRequest is the payload for registering a new manufacturer.
type RegisterManufacturerRequest struct {
	Name        string `json:"name" example:"Tata Motors EV Division"`
	CountryCode string `json:"country_code" example:"IN"`
	ProfileData string `json:"profile_data" example:"{\"address\":\"Mumbai, India\",\"contact\":\"+91-22-XXXXXXX\"}"`
}

// RegisterManufacturerResponse is returned after successful manufacturer registration.
type RegisterManufacturerResponse struct {
	ID               string `json:"id"`
	ManufacturerCode string `json:"manufacturer_code"`
	Name             string `json:"name"`
}

// ManufacturerProfile is the full manufacturer profile (decrypted).
type ManufacturerProfile struct {
	ID               string    `json:"id"`
	ManufacturerCode string    `json:"manufacturer_code"`
	Name             string    `json:"name"`
	CountryCode      string    `json:"country_code"`
	ProfileData      string    `json:"profile_data"`
	CreatedAt        time.Time `json:"created_at"`
}

// BatteryCsvRow is one row in a batch battery registration CSV.
type BatteryCsvRow struct {
	ChemistryType    string  `json:"chemistry_type"`
	BatteryCategory  string  `json:"battery_category"`
	ComplianceClass  string  `json:"compliance_class"`
	NominalVoltage   float64 `json:"nominal_voltage"`
	RatedCapacityKwh float64 `json:"rated_capacity_kwh"`
	EnergyDensity    float64 `json:"energy_density"`
	WeightKg         float64 `json:"weight_kg"`
	FormFactor       string  `json:"form_factor"`
	SerialNumber     string  `json:"serial_number"`
	BatchNumber      string  `json:"batch_number"`
	FactoryCode      string  `json:"factory_code"`
	ProductionYear   int     `json:"production_year"`
	SequenceNumber   string  `json:"sequence_number"`
}

// BatchBatteryRequest is the payload for batch battery registration.
type BatchBatteryRequest struct {
	ManufacturerCode string         `json:"manufacturer_code"`
	Batteries        []BatteryCsvRow `json:"batteries"`
}

// BatteryBatchResult is the result for one battery in a batch.
type BatteryBatchResult struct {
	BPAN       string `json:"bpan"`
	StaticHash string `json:"static_hash"`
	Status     string `json:"status"`
}

// BatchBatteryResponse is returned after batch battery registration.
type BatchBatteryResponse struct {
	ManufacturerID string               `json:"manufacturer_id"`
	Total          int                  `json:"total"`
	Batteries      []BatteryBatchResult `json:"batteries"`
	AuditID        string               `json:"audit_id"`
}

// ManufacturerBatterySummary is a compact view of a battery owned by a manufacturer.
type ManufacturerBatterySummary struct {
	BPAN               string  `json:"bpan"`
	ChemistryType      string  `json:"chemistry_type"`
	BatteryCategory    string  `json:"battery_category"`
	RatedCapacityKwh   float64 `json:"rated_capacity_kwh"`
	NominalVoltage     float64 `json:"nominal_voltage"`
	StateOfHealth      float64 `json:"state_of_health"`
	TotalCycles        int     `json:"total_cycles"`
	RegistrationStatus string  `json:"registration_status"`
	ProductionYear     int     `json:"production_year"`
}

// ManufacturerDashboard holds aggregated dashboard data for a manufacturer.
type ManufacturerDashboard struct {
	TotalBatteries         int64   `json:"total_batteries"`
	Operational            int64   `json:"operational"`
	PendingRegistrations   int64   `json:"pending_registrations"`
	RejectedRegistrations  int64   `json:"rejected_registrations"`
	SecondLife             int64   `json:"second_life"`
	EndOfLife              int64   `json:"end_of_life"`
	AverageSoH             float64 `json:"average_soh"`
	ComplianceViolations   int64   `json:"compliance_violations"`
}
