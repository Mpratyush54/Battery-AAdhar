// health.go — Health models for API

package models

import "time"

type HealthUpdateRequest struct {
	StateOfHealthPercent      float32  `json:"state_of_health_percent"`
	CycleCount                uint32   `json:"cycle_count"`
	DegradationClass          string   `json:"degradation_class"`
	MinTemperatureCelsius     *float32 `json:"min_temperature_celsius,omitempty"`
	MaxTemperatureCelsius     *float32 `json:"max_temperature_celsius,omitempty"`
	AverageTemperatureCelsius *float32 `json:"average_temperature_celsius,omitempty"`
	CellVoltageMinMv          *float32 `json:"cell_voltage_min_mv,omitempty"`
	CellVoltageMaxMv          *float32 `json:"cell_voltage_max_mv,omitempty"`
	InternalResistanceMohm    *float32 `json:"internal_resistance_mohm,omitempty"`
	ErrorFlags                *string  `json:"error_flags,omitempty"`
}

type HealthRecord struct {
	BPAN                 string    `json:"bpan"`
	StateOfHealthPercent float32   `json:"state_of_health_percent"`
	HealthStatus         string    `json:"health_status"` // OPERATIONAL, SECOND_LIFE, etc.
	CycleCount           uint32    `json:"cycle_count"`
	Temperature          float32   `json:"average_temperature_celsius"`
	ProofsGenerated      bool      `json:"proofs_generated"`
	ReportedAt           time.Time `json:"reported_at"`
}

type HealthDashboard struct {
	AvgSohByManufacturer map[string]float32 `json:"avg_soh_by_manufacturer"`
	AvgSohByChemistry    map[string]float32 `json:"avg_soh_by_chemistry"`
	OperationalCount     int                `json:"operational_count"`
	SecondLifeCount      int                `json:"second_life_count"`
	EolCount             int                `json:"eol_count"`
	WasteCount           int                `json:"waste_count"`
}
