package models

type RegisterBatteryPayload struct {
	ManufacturerID   string  `json:"manufacturerId" example:"123e4567-e89b-12d3-a456-426614174000"`
	ManufacturerCode string  `json:"manufacturerCode" example:"ABC"`
	ChemistryType    string  `json:"chemistryType" example:"LFP"`
	BatteryCategory  string  `json:"batteryCategory" example:"EV-M"`
	ComplianceClass  string  `json:"complianceClass" example:"AIS-156"`
	NominalVoltage   float64 `json:"nominalVoltage" example:"48.0"`
	RatedCapacityKwh float64 `json:"ratedCapacityKwh" example:"2.5"`
	EnergyDensity    float64 `json:"energyDensity" example:"150.0"`
	WeightKg         float64 `json:"weightKg" example:"25.0"`
	FormFactor       string  `json:"formFactor" example:"PRISMATIC"`
	SerialNumber     string  `json:"serialNumber" example:"SN12345678"`
	BatchNumber      string  `json:"batchNumber" example:"BAT2026"`
	FactoryCode      string  `json:"factoryCode" example:"FAC01"`
	ProductionYear   uint32  `json:"productionYear" example:"2026"`
	SequenceNumber   string  `json:"sequenceNumber" example:"01"`
	ActorID          string  `json:"actorId" example:"123e4567-e89b-12d3-a456-426614174000"`
}

type RegisterBatteryResponseJSON struct {
	Bpan           string `json:"bpan"`
	StaticHash     string `json:"staticHash"`
	RegistrationId string `json:"registrationId"`
	Status         string `json:"status"`
}
