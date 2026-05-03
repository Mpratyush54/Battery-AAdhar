// carbon.go — Carbon Footprint models

package models

import "time"

type CarbonFootprintRequest struct {
	RawMaterialEmissionsKgCo2e   float32 `json:"raw_material_emissions_kg_co2e"`
	RawMaterialSourceCountry     string  `json:"raw_material_source_country"`
	MiningMethod                 string  `json:"mining_method"`
	ManufacturingEmissionsKgCo2e float32 `json:"manufacturing_emissions_kg_co2e"`
	ManufacturingLocation        string  `json:"manufacturing_location"`
	FactoryEnergySource          string  `json:"factory_energy_source"`
	CellProductionMethod         string  `json:"cell_production_method"`
	TransportEmissionsKgCo2e     float32 `json:"transport_emissions_kg_co2e"`
	TransportDistanceKm          float32 `json:"transport_distance_km"`
	TransportMode                string  `json:"transport_mode"`
	TransportPackaging           string  `json:"transport_packaging"`
	UsageEmissionsKgCo2e         float32 `json:"usage_emissions_kg_co2e"`
	UsageYears                   int     `json:"usage_years"`
	UsageGridEmissionsFactorGCo2ePerKwh float32 `json:"usage_grid_emissions_factor_gco2e_per_kwh"`
	UsageAnnualKm                int     `json:"usage_annual_km"`
	RecyclingEmissionsKgCo2e     float32 `json:"recycling_emissions_kg_co2e"`
	RecyclingRecoveryRate        float32 `json:"recycling_recovery_rate"`
	RecyclingAvoidedMining       float32 `json:"recycling_avoided_mining"`
	RecyclingMethod              string  `json:"recycling_method"`
}

type CarbonFootprintResponse struct {
	BPAN                  string     `json:"bpan"`
	TotalEmissionsKgCo2e  float32    `json:"total_emissions_kg_co2e"`
	EmissionsPerKwh       float32    `json:"emissions_per_kwh"`
	Verified              bool       `json:"verified"`
	VerifiedBy            *string    `json:"verified_by,omitempty"`
	VerifiedAt            *time.Time `json:"verified_at,omitempty"`
	VerificationStandard  *string    `json:"verification_standard,omitempty"`
}

type CarbonComparison struct {
	BpanA                 string  `json:"bpan_a"`
	BpanB                 string  `json:"bpan_b"`
	Stage1Delta           float32 `json:"stage1_delta"`
	Stage2Delta           float32 `json:"stage2_delta"`
	Stage3Delta           float32 `json:"stage3_delta"`
	Stage4Delta           float32 `json:"stage4_delta"`
	Stage5Delta           float32 `json:"stage5_delta"`
	TotalDelta            float32 `json:"total_delta"`
	EmissionsPerKwhDelta  float32 `json:"emissions_per_kwh_delta"`
	BpanALower            bool    `json:"bpan_a_lower"`
}

type VerificationRequest struct {
	Standard string `json:"standard"` // ISO 14040, PEF, EU ETS
}
