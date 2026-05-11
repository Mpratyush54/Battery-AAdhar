// reuse_recycling_service.go — Circular Economy Service Layer (Go)
// This service orchestrates requests between the API controllers and the Rust gRPC core.
// It handles business logic validation and data transformation.

package services

import (
	"context"
	"fmt"

	circular_economyv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/circular_economy/v1"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

// ReuseService manages second-life certification for batteries.
type ReuseService struct {
	grpcClient circular_economyv1.CircularEconomyServiceClient
}

// NewReuseService creates a new instance of ReuseService using the provided gRPC connection.
func NewReuseService(cc *grpc.ClientConn) *ReuseService {
	return &ReuseService{
		grpcClient: cc.CircularEconomyClient,
	}
}

// CertifySecondLife validates the SoH and application for a battery and issues a second-life certificate.
// Eligibility: SoH must be between 60% and 80%.
func (s *ReuseService) CertifySecondLife(
	ctx context.Context,
	bpan string,
	soh float32,
	certifiedBy string,
	application string,
	expectedYears uint8,
) (string, error) {
	// Validate SoH is in second-life range (60–80%)
	if soh < 60.0 || soh > 80.0 {
		return "", fmt.Errorf("SoH must be between 60.0%% and 80.0%% for second-life certification, got %.1f%%", soh)
	}

	// Call Rust core via gRPC
	resp, err := s.grpcClient.CertifyReuse(ctx, &circular_economyv1.CertifyReuseRequest{
		Bpan:          bpan,
		SohPercent:    soh,
		CertifiedBy:   certifiedBy,
		Application:   application,
		ExpectedYears: uint32(expectedYears),
	})
	if err != nil {
		return "", fmt.Errorf("failed to certify reuse via gRPC: %w", err)
	}

	return resp.CertificationId, nil
}

// RecyclingService manages the recording of battery recycling and material recovery rates.
type RecyclingService struct {
	grpcClient circular_economyv1.CircularEconomyServiceClient
}

// NewRecyclingService creates a new instance of RecyclingService.
func NewRecyclingService(cc *grpc.ClientConn) *RecyclingService {
	return &RecyclingService{
		grpcClient: cc.CircularEconomyClient,
	}
}

// RecordRecycling records a battery's end-of-life recycling event with material recovery metrics.
func (s *RecyclingService) RecordRecycling(
	ctx context.Context,
	bpan string,
	recycledBy string,
	method string,
	weightKg float32,
	standard string,
	recovery models.RecoveryRates,
) (string, error) {
	// Validate recovery rates (0–100%)
	if recovery.LithiumPercent < 0 || recovery.LithiumPercent > 100 {
		return "", fmt.Errorf("invalid Lithium recovery rate: %.1f%%", recovery.LithiumPercent)
	}
	if recovery.CobaltPercent < 0 || recovery.CobaltPercent > 100 {
		return "", fmt.Errorf("invalid Cobalt recovery rate: %.1f%%", recovery.CobaltPercent)
	}
	if recovery.NickelPercent < 0 || recovery.NickelPercent > 100 {
		return "", fmt.Errorf("invalid Nickel recovery rate: %.1f%%", recovery.NickelPercent)
	}

	// Call Rust core via gRPC
	resp, err := s.grpcClient.RecordRecycling(ctx, &circular_economyv1.RecordRecyclingRequest{
		Bpan:       bpan,
		RecycledBy: recycledBy,
		Method:     method,
		WeightKg:   weightKg,
		Standard:   standard,
		RecoveryRates: &circular_economyv1.RecoveryRates{
			LithiumPercent: recovery.LithiumPercent,
			CobaltPercent:  recovery.CobaltPercent,
			NickelPercent:  recovery.NickelPercent,
			OtherPercent:   recovery.OtherPercent,
		},
	})
	if err != nil {
		return "", fmt.Errorf("failed to record recycling via gRPC: %w", err)
	}

	return resp.CertificationId, nil
}

// DashboardService provides aggregated metrics for the circular economy.
type DashboardService struct {
	grpcClient circular_economyv1.CircularEconomyServiceClient
}

// NewDashboardService creates a new instance of DashboardService.
func NewDashboardService(cc *grpc.ClientConn) *DashboardService {
	return &DashboardService{
		grpcClient: cc.CircularEconomyClient,
	}
}

// GetCircularEconomyMetrics retrieves aggregated recovery metrics, optionally filtered by manufacturer or chemistry.
func (s *DashboardService) GetCircularEconomyMetrics(
	ctx context.Context,
	manufacturerID string,
	chemistryType string,
) (*models.CircularEconomyDashboard, error) {
	// Call Rust core via gRPC
	resp, err := s.grpcClient.GetMetrics(ctx, &circular_economyv1.GetMetricsRequest{
		ManufacturerId: manufacturerID,
		ChemistryType:  chemistryType,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to fetch metrics via gRPC: %w", err)
	}

	if resp.Metrics == nil {
		return nil, fmt.Errorf("received empty metrics from gRPC engine")
	}

	// Transform gRPC response to API model
	dashboard := &models.CircularEconomyDashboard{
		ByManufacturer: make(map[string]models.CircularEconomyMetrics),
		ByChemistry:    make(map[string]models.CircularEconomyMetrics),
		Overall: models.CircularEconomyMetrics{
			BatteryCount:           resp.Metrics.BatteryCount,
			TotalWeightProcessedKg: resp.Metrics.TotalWeightProcessedKg,
			AvgLiRecovery:          resp.Metrics.AvgLiRecovery,
			AvgCoRecovery:          resp.Metrics.AvgCoRecovery,
			AvgNiRecovery:          resp.Metrics.AvgNiRecovery,
		},
	}

	// If filtered, populate the specific map entry for clarity
	if manufacturerID != "" {
		dashboard.ByManufacturer[manufacturerID] = dashboard.Overall
	}
	if chemistryType != "" {
		dashboard.ByChemistry[chemistryType] = dashboard.Overall
	}

	return dashboard, nil
}
