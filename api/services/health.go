// health.go — Health service orchestration

package services

import (
	"context"
	"fmt"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

type HealthService struct {
	// TODO Day 7: Add Rust gRPC client for ZK proof generation
}

func NewHealthService() *HealthService {
	return &HealthService{}
}

func (s *HealthService) UpdateHealth(
	ctx context.Context,
	bpan string,
	req *models.HealthUpdateRequest,
	role string,
) (string, error) {
	if role != "bms" && role != "manufacturer" && role != "admin" {
		return "", fmt.Errorf("unauthorized role")
	}

	if req.StateOfHealthPercent < 0 || req.StateOfHealthPercent > 100 {
		return "", fmt.Errorf("invalid SoH")
	}

	// TODO Day 7: Call Rust service to generate ZK proofs + store in DB
	recordID := fmt.Sprintf("rec-%s", bpan)
	return recordID, nil
}

func (s *HealthService) GetCurrentHealth(
	ctx context.Context,
	bpan string,
) (*models.HealthRecord, error) {
	// TODO Day 7: Fetch from DB
	return &models.HealthRecord{
		BPAN:                 bpan,
		StateOfHealthPercent: 85.5,
		HealthStatus:         "OPERATIONAL",
	}, nil
}

func (s *HealthService) GetHealthHistory(
	ctx context.Context,
	bpan string,
	limit int32,
) ([]*models.HealthRecord, error) {
	// TODO Day 7: Fetch from DB
	return []*models.HealthRecord{}, nil
}

func (s *HealthService) GetDashboard(ctx context.Context) (*models.HealthDashboard, error) {
	// TODO Day 7: Query aggregations
	return &models.HealthDashboard{
		AvgSohByManufacturer: make(map[string]float32),
		AvgSohByChemistry:    make(map[string]float32),
	}, nil
}
