// health.go — Battery health service layer
// Orchestrates gRPC calls to the Rust core for health operations.

package services

import (
	"context"
	"fmt"
	"log/slog"
	"time"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
	healthv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/health/v1"
)

// HealthService handles battery health operations.
type HealthService struct {
	healthClient healthv1.HealthServiceClient
}

// NewHealthService creates a new health service.
func NewHealthService() *HealthService {
	return &HealthService{}
}

// NewHealthServiceWithClient creates a health service with gRPC client.
func NewHealthServiceWithClient(cc *grpc.ClientConn) *HealthService {
	if cc == nil {
		return NewHealthService()
	}
	return &HealthService{
		healthClient: cc.HealthClient,
	}
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

	if s.healthClient != nil {
		grpcReq := &healthv1.UpdateHealthRequest{
			Bpan:                 bpan,
			StateOfHealthPercent: float64(req.StateOfHealthPercent),
			CycleCount:           int32(req.CycleCount),
			DegradationClass:     req.DegradationClass,
			RequesterId:          requesterRole,
		}
		if req.MinTemperatureCelsius != nil {
			grpcReq.MinTemperatureCelsius = float64(*req.MinTemperatureCelsius)
		}
		if req.MaxTemperatureCelsius != nil {
			grpcReq.MaxTemperatureCelsius = float64(*req.MaxTemperatureCelsius)
		}
		if req.AverageTemperatureCelsius != nil {
			grpcReq.AverageTemperatureCelsius = float64(*req.AverageTemperatureCelsius)
		}
		if req.CellVoltageMinMv != nil {
			grpcReq.CellVoltageMinMv = float64(*req.CellVoltageMinMv)
		}
		if req.CellVoltageMaxMv != nil {
			grpcReq.CellVoltageMaxMv = float64(*req.CellVoltageMaxMv)
		}
		if req.InternalResistanceMohm != nil {
			grpcReq.InternalResistanceMohm = float64(*req.InternalResistanceMohm)
		}
		if req.ErrorFlags != nil {
			grpcReq.ErrorFlags = *req.ErrorFlags
		}

		resp, err := s.healthClient.UpdateHealth(ctx, grpcReq)
		if err != nil {
			return "", fmt.Errorf("gRPC error: %w", err)
		}
		return resp.RecordId, nil
	}

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

	if s.healthClient != nil {
		resp, err := s.healthClient.GetHealth(ctx, &healthv1.GetHealthRequest{
			Bpan: bpan,
		})
		if err != nil {
			return nil, fmt.Errorf("gRPC error: %w", err)
		}

		rec := resp.Record
		var reportedAt time.Time
		if rec.ReportedAt != nil {
			reportedAt = time.Unix(rec.ReportedAt.Seconds, int64(rec.ReportedAt.Nanos))
		}

		return &models.HealthRecord{
			BPAN:                 rec.Bpan,
			StateOfHealthPercent: float32(rec.StateOfHealthPercent),
			HealthStatus:         rec.HealthStatus,
			CycleCount:           uint32(rec.CycleCount),
			Temperature:          float32(rec.AverageTemperatureCelsius),
			ReportedAt:           reportedAt,
		}, nil
	}

	return &models.HealthRecord{
		BPAN:                 bpan,
		StateOfHealthPercent: 100.0,
		HealthStatus:         "OPERATIONAL",
	}, nil
}

// GetHealthHistory retrieves health history.
func (s *HealthService) GetHealthHistory(
	ctx context.Context,
	bpan string,
	limit int,
) ([]*models.HealthRecord, error) {
	slog.Info("fetching health history", "bpan", bpan, "limit", limit)

	if s.healthClient != nil {
		resp, err := s.healthClient.GetHealthHistory(ctx, &healthv1.GetHealthHistoryRequest{
			Bpan:  bpan,
			Limit: int32(limit),
		})
		if err != nil {
			return nil, fmt.Errorf("gRPC error: %w", err)
		}

		records := make([]*models.HealthRecord, len(resp.Records))
		for i, rec := range resp.Records {
			var reportedAt time.Time
			if rec.ReportedAt != nil {
				reportedAt = time.Unix(rec.ReportedAt.Seconds, int64(rec.ReportedAt.Nanos))
			}
			records[i] = &models.HealthRecord{
				BPAN:                 rec.Bpan,
				StateOfHealthPercent: float32(rec.StateOfHealthPercent),
				HealthStatus:         rec.HealthStatus,
				CycleCount:           uint32(rec.CycleCount),
				Temperature:          float32(rec.AverageTemperatureCelsius),
				ReportedAt:           reportedAt,
			}
		}
		return records, nil
	}

	return []*models.HealthRecord{}, nil
}

// GetAvgSoHByManufacturer retrieves average SoH by manufacturer.
func (s *HealthService) GetAvgSoHByManufacturer(
	ctx context.Context,
	manufacturerID string,
) (float32, error) {
	slog.Info("fetching avg SoH by manufacturer", "manufacturer_id", manufacturerID)

	if s.healthClient != nil {
		resp, err := s.healthClient.GetAvgSoH(ctx, &healthv1.GetAvgSoHRequest{
			ManufacturerId: manufacturerID,
		})
		if err != nil {
			return 0, fmt.Errorf("gRPC error: %w", err)
		}
		return float32(resp.AvgSohPercent), nil
	}

	return 0, nil
}

// GetAvgSoHByChemistry retrieves average SoH by chemistry type.
func (s *HealthService) GetAvgSoHByChemistry(
	ctx context.Context,
	chemistryType string,
) (float32, error) {
	slog.Info("fetching avg SoH by chemistry", "chemistry", chemistryType)

	if s.healthClient != nil {
		resp, err := s.healthClient.GetAvgSoH(ctx, &healthv1.GetAvgSoHRequest{
			ChemistryType: chemistryType,
		})
		if err != nil {
			return 0, fmt.Errorf("gRPC error: %w", err)
		}
		return float32(resp.AvgSohPercent), nil
	}

	return 0, nil
}

// GetHealthDashboard retrieves aggregated health dashboard metrics.
func (s *HealthService) GetHealthDashboard(
	ctx context.Context,
) (*models.HealthDashboard, error) {
	slog.Info("fetching health dashboard")

	if s.healthClient != nil {
		resp, err := s.healthClient.GetDashboard(ctx, &healthv1.HealthDashboardRequest{})
		if err != nil {
			return nil, fmt.Errorf("gRPC error: %w", err)
		}

		avgMfr := make(map[string]float32, len(resp.AvgSohByManufacturer))
		for k, v := range resp.AvgSohByManufacturer {
			avgMfr[k] = float32(v)
		}
		avgChem := make(map[string]float32, len(resp.AvgSohByChemistry))
		for k, v := range resp.AvgSohByChemistry {
			avgChem[k] = float32(v)
		}

		return &models.HealthDashboard{
			AvgSohByManufacturer: avgMfr,
			AvgSohByChemistry:    avgChem,
		}, nil
	}

	return &models.HealthDashboard{
		AvgSohByManufacturer: map[string]float32{},
		AvgSohByChemistry:    map[string]float32{},
	}, nil
}
