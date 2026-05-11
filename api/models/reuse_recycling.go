package models

type ReuseCertificationRequest struct {
	SohPercent      float32 `json:"soh_percent"`
	Application     string  `json:"application"` // "stationary_storage", "grid_backup", "renewable_integration"
	ExpectedYears   uint8   `json:"expected_years"`
}

type RecoveryRates struct {
	LithiumPercent float32 `json:"lithium_percent"`
	CobaltPercent  float32 `json:"cobalt_percent"`
	NickelPercent  float32 `json:"nickel_percent"`
	OtherPercent   float32 `json:"other_percent"`
}

type RecyclingRecordRequest struct {
	Method        string       `json:"method"` // "hydrometallurgical", "pyrometallurgical", "mechanical"
	WeightKg      float32      `json:"weight_kg"`
	Standard      string       `json:"standard"` // "ISO 14040", "R2C2", etc.
	RecoveryRates RecoveryRates `json:"recovery_rates"`
}

type CircularEconomyMetrics struct {
	BatteryCount            uint32  `json:"battery_count"`
	AvgLiRecovery           float32 `json:"avg_li_recovery_percent"`
	AvgCoRecovery           float32 `json:"avg_co_recovery_percent"`
	AvgNiRecovery           float32 `json:"avg_ni_recovery_percent"`
	TotalWeightProcessedKg  float32 `json:"total_weight_processed_kg"`
}

type CircularEconomyDashboard struct {
	ByManufacturer map[string]CircularEconomyMetrics `json:"by_manufacturer"`
	ByChemistry    map[string]CircularEconomyMetrics `json:"by_chemistry"`
	Overall        CircularEconomyMetrics             `json:"overall"`
}
