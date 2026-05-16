// health.go — Battery health service layer
// Orchestrates gRPC calls to the Rust core for health operations.

package services

import (
	"context"
	"fmt"
	"log/slog"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

// HealthService handles battery health operations.
// Currently local-only — Rust gRPC for health will be added when proto expands.
type HealthService struct{}

// NewHealthService creates a new health service.
func NewHealthService() *HealthService {
	return &HealthService{}
}

// UpdateHealth updates battery health data.
func (s *HealthService) UpdateHealth(
	ctx context.Context,
	bpan string,
	req *models.HealthUpdateRequest,
	requesterRole string,
) (string, error) {
	if bpan == "" {
		return "", fmt.Errorf("bpan is required")
	}
	if req.StateOfHealthPercent < 0 || req.StateOfHealthPercent > 100 {
		return "", fmt.Errorf("SoH must be 0-100")
	}

	slog.Info("updating health",
		"bpan", bpan,
		"soh", req.StateOfHealthPercent,
		"role", requesterRole,
	)

	// TODO: Wire to Rust gRPC when health proto is added
	return fmt.Sprintf("health:%s", bpan), nil
}

// GetCurrentHealth retrieves current health status.
func (s *HealthService) GetCurrentHealth(
	ctx context.Context,
	bpan string,
) (*models.HealthRecord, error) {
	if bpan == "" {
		return nil, fmt.Errorf("bpan is required")
	}

	slog.Info("fetching current health", "bpan", bpan)

	// TODO: Wire to Rust gRPC
	return &models.HealthRecord{
		BPAN:                bpan,
		StateOfHealthPercent: 100.0,
		HealthStatus:        "OPERATIONAL",
	}, nil
}

// GetHealthHistory retrieves health history.
func (s *HealthService) GetHealthHistory(
	ctx context.Context,
	bpan string,
	limit int,
) ([]*models.HealthRecord, error) {
	slog.Info("fetching health history", "bpan", bpan, "limit", limit)

	// TODO: Wire to Rust gRPC
	return []*models.HealthRecord{}, nil
}

// GetAvgSoHByManufacturer retrieves average SoH by manufacturer.
func (s *HealthService) GetAvgSoHByManufacturer(
	ctx context.Context,
	manufacturerID string,
) (float32, error) {
	slog.Info("fetching avg SoH by manufacturer", "manufacturer_id", manufacturerID)

	// TODO: Wire to Rust gRPC
	return 0, nil
}

// GetAvgSoHByChemistry retrieves average SoH by chemistry type.
func (s *HealthService) GetAvgSoHByChemistry(
	ctx context.Context,
	chemistryType string,
) (float32, error) {
	slog.Info("fetching avg SoH by chemistry", "chemistry", chemistryType)

	// TODO: Wire to Rust gRPC
	return 0, nil
}
