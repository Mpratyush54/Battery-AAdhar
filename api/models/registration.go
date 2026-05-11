package models

// BatteryDescriptorRequest represents the static descriptor for a battery
type BatteryDescriptorRequest struct {
	CapacityKwh          float64 `json:"capacity_kwh"`
	NominalVoltageV      float64 `json:"nominal_voltage_v"`
	NominalCurrentA      float64 `json:"nominal_current_a"`
	ChemistryType        string  `json:"chemistry_type"`
	CellType             string  `json:"cell_type"`
	CellCount            uint8   `json:"cell_count"`
	CellVoltageNominalV  float64 `json:"cell_voltage_nominal_v"`
	ManufacturerId       string  `json:"manufacturer_id"`
	ManufacturingCountry string  `json:"manufacturing_country"`
	ManufacturingFacility string `json:"manufacturing_facility"`
	ManufactureDate      string  `json:"manufacture_date"`
	DeclaredCycleLife    uint32  `json:"declared_cycle_life"`
	WarrantyYears        uint8   `json:"warranty_years"`
}

// BatteryRegistrationRequest is the full payload for registering a battery
type BatteryRegistrationRequest struct {
	Descriptor   BatteryDescriptorRequest `json:"descriptor"`
	Material     MaterialCompositionRequest `json:"material"`
	Carbon       CarbonFootprintRequest   `json:"carbon"`
	Health       HealthUpdateRequest      `json:"health"`
}
