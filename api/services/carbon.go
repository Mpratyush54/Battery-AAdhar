// carbon.go — Carbon footprint service layer
// Orchestrates gRPC calls to the Rust core for BCF operations.

package services

import (
	"context"
	"fmt"
	"log/slog"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

// CarbonService handles BCF operations.
// Currently local-only — Rust gRPC for carbon will be added when proto expands.
type CarbonService struct{}

// NewCarbonService creates a new carbon service.
func NewCarbonService() *CarbonService {
	return &CarbonService{}
}

// SubmitCarbonFootprint validates and stores carbon footprint data.
func (s *CarbonService) SubmitCarbonFootprint(
	ctx context.Context,
	bpan string,
	submitterID string,
	req *models.CarbonFootprintRequest,
) (string, error) {
	if bpan == "" || submitterID == "" {
		return "", fmt.Errorf("bpan and submitter_id are required")
	}
	if req.RawMaterialEmissionsKgCo2e < 0 || req.ManufacturingEmissionsKgCo2e < 0 {
		return "", fmt.Errorf("emissions must be non-negative")
	}

	slog.Info("submitting BCF",
		"bpan", bpan,
		"submitter_id", submitterID,
		"total", req.RawMaterialEmissionsKgCo2e+req.ManufacturingEmissionsKgCo2e+req.TransportEmissionsKgCo2e+req.UsageEmissionsKgCo2e+req.RecyclingEmissionsKgCo2e,
	)

	// TODO: Wire to Rust gRPC when carbon proto is added
	return fmt.Sprintf("bcf:%s:%s", bpan, submitterID), nil
}

// VerifyCarbonFootprint marks a carbon footprint as verified.
func (s *CarbonService) VerifyCarbonFootprint(
	ctx context.Context,
	bpan string,
	verifiedBy string,
	standard string,
) error {
	if bpan == "" || verifiedBy == "" {
		return fmt.Errorf("bpan and verified_by are required")
	}

	slog.Info("verifying BCF",
		"bpan", bpan,
		"verified_by", verifiedBy,
		"standard", standard,
	)

	// TODO: Wire to Rust gRPC
	return nil
}

// GetCarbonFootprint retrieves carbon footprint data.
func (s *CarbonService) GetCarbonFootprint(
	ctx context.Context,
	bpan string,
	requesterRole string,
) (*models.CarbonFootprintResponse, error) {
	if bpan == "" {
		return nil, fmt.Errorf("bpan is required")
	}

	slog.Info("fetching BCF", "bpan", bpan, "role", requesterRole)

	// TODO: Wire to Rust gRPC
	return &models.CarbonFootprintResponse{
		BPAN:              bpan,
		TotalEmissionsKgCo2e: 0,
		Verified:          false,
	}, nil
}

// CompareCarbonFootprints compares two batteries' carbon footprints.
func (s *CarbonService) CompareCarbonFootprints(
	ctx context.Context,
	bpanA, bpanB string,
) (*models.CarbonComparison, error) {
	slog.Info("comparing carbon footprints", "bpan_a", bpanA, "bpan_b", bpanB)

	// TODO: Wire to Rust gRPC
	return &models.CarbonComparison{
		BpanA:     bpanA,
		BpanB:     bpanB,
		TotalDelta: 0,
	}, nil
}
