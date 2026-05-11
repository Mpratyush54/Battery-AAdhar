// registration.go — Battery Registration orchestration service

package services

import (
	"context"
	"fmt"
	
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	batteryv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/battery/v1"
	"google.golang.org/grpc"
)

// RegistrationService orchestrates the atomic battery registration via Rust core
type RegistrationService struct {
	grpcClient batteryv1.BatteryServiceClient
}

// NewRegistrationService creates a new registration service
func NewRegistrationService(conn *grpc.ClientConn) *RegistrationService {
	return &RegistrationService{
		grpcClient: batteryv1.NewBatteryServiceClient(conn),
	}
}

// RegisterBattery calls the Rust core service to perform an atomic registration.
// It links descriptor, BMCS, BCF, initial health, and creates BPAN in one transaction.
func (s *RegistrationService) RegisterBattery(
	ctx context.Context,
	req *models.BatteryRegistrationRequest,
	requesterID string,
) (string, error) {
	
	if req.Descriptor.CapacityKwh <= 0 {
		return "", fmt.Errorf("invalid capacity: must be > 0")
	}

	grpcReq := &batteryv1.RegisterBatteryRequest{
		ManufacturerId: req.Descriptor.ManufacturerId,
		StaticData: &batteryv1.BatteryStaticData{
			CountryCode: req.Descriptor.ManufacturingCountry,
			ManufacturerCode: "UNK", // Fallback, could be added to payload
			BatteryCapacityKwh: float32(req.Descriptor.CapacityKwh),
			BatteryChemistry: req.Descriptor.ChemistryType,
			NominalVoltage: float32(req.Descriptor.NominalVoltageV),
			CellOrigin: "UNK",
			ExtinguisherClass: "UNK",
			ManufacturingDate: req.Descriptor.ManufactureDate,
			FactoryCode: req.Descriptor.ManufacturingFacility,
			SequentialNumber: "00001",
			NumCells: int32(req.Descriptor.CellCount),
			WarrantyYears: int32(req.Descriptor.WarrantyYears),
			CellType: req.Descriptor.CellType,
		},
		Material: &batteryv1.MaterialComposition{
			CathodeMaterial: req.Material.CathodeMaterial,
			AnodeMaterial: req.Material.AnodeMaterial,
			ElectrolyteType: req.Material.ElectrolyteType,
			SeparatorMaterial: req.Material.SeparatorMaterial,
			RecyclablePercentage: req.Material.RecyclablePercent,
			LithiumContentG: req.Material.LithiumContentG,
			CobaltContentG: req.Material.CobaltContentG,
			NickelContentG: req.Material.NickelContentG,
			ManganeseContentG: req.Material.ManganeseContentG,
			LeadContentG: req.Material.LeadContentG,
			CadmiumContentG: req.Material.CadmiumContentG,
			HazardousSubstances: req.Material.HazardousSubstances,
			SupplyChainSource: req.Material.SupplyChainSource,
		},
		Carbon: &batteryv1.CarbonFootprint{
			RawMaterialEmissionsKgCo2E: float64(req.Carbon.RawMaterialEmissionsKgCo2e),
			ManufacturingEmissionsKgCo2E: float64(req.Carbon.ManufacturingEmissionsKgCo2e),
			TransportEmissionsKgCo2E: float64(req.Carbon.TransportEmissionsKgCo2e),
			UsageEmissionsKgCo2E: float64(req.Carbon.UsageEmissionsKgCo2e),
			RecyclingEmissionsKgCo2E: float64(req.Carbon.RecyclingEmissionsKgCo2e),
			TotalEmissionsKgCo2E: float64(req.Carbon.RawMaterialEmissionsKgCo2e + req.Carbon.ManufacturingEmissionsKgCo2e + req.Carbon.TransportEmissionsKgCo2e + req.Carbon.UsageEmissionsKgCo2e + req.Carbon.RecyclingEmissionsKgCo2e),
			Verified: false,
		},
		InitialHealth: &batteryv1.HealthRecord{
			StateOfHealthPercent: float64(req.Health.StateOfHealthPercent),
			CycleCount: int32(req.Health.CycleCount),
			DegradationClass: req.Health.DegradationClass,
		},
	}

	resp, err := s.grpcClient.RegisterBattery(ctx, grpcReq)
	if err != nil {
		return "", fmt.Errorf("failed to register battery via gRPC: %w", err)
	}

	return resp.Bpan, nil
}

