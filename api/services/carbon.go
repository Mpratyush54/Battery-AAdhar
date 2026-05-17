// carbon.go — Carbon footprint service layer
// Orchestrates gRPC calls to the Rust core for BCF operations.

package services

import (
	"context"
	"fmt"
	"log/slog"
	"time"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
	carbonv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/carbon/v1"
)

// CarbonService handles BCF operations.
type CarbonService struct {
	carbonClient carbonv1.CarbonServiceClient
}

// NewCarbonService creates a new carbon service.
func NewCarbonService() *CarbonService {
	return &CarbonService{}
}

// NewCarbonServiceWithClient creates a carbon service with gRPC client.
func NewCarbonServiceWithClient(cc *grpc.ClientConn) *CarbonService {
	if cc == nil {
		return NewCarbonService()
	}
	return &CarbonService{
		carbonClient: cc.CarbonClient,
	}
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

	if s.carbonClient != nil {
		resp, err := s.carbonClient.SubmitCarbon(ctx, &carbonv1.SubmitCarbonRequest{
			Bpan:                        bpan,
			RawMaterialEmissionsKgCo2E:  float64(req.RawMaterialEmissionsKgCo2e),
			ManufacturingEmissionsKgCo2E: float64(req.ManufacturingEmissionsKgCo2e),
			TransportEmissionsKgCo2E:    float64(req.TransportEmissionsKgCo2e),
			UsageEmissionsKgCo2E:        float64(req.UsageEmissionsKgCo2e),
			RecyclingEmissionsKgCo2E:    float64(req.RecyclingEmissionsKgCo2e),
			SubmitterId:                 submitterID,
		})
		if err != nil {
			return "", fmt.Errorf("gRPC error: %w", err)
		}
		return resp.CarbonId, nil
	}

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

	if s.carbonClient != nil {
		_, err := s.carbonClient.VerifyCarbon(ctx, &carbonv1.VerifyCarbonRequest{
			Bpan:       bpan,
			VerifiedBy: verifiedBy,
			Standard:   standard,
		})
		if err != nil {
			return fmt.Errorf("gRPC error: %w", err)
		}
	}

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

	if s.carbonClient != nil {
		resp, err := s.carbonClient.GetCarbon(ctx, &carbonv1.GetCarbonRequest{
			Bpan:         bpan,
			RequesterRole: requesterRole,
		})
		if err != nil {
			return nil, fmt.Errorf("gRPC error: %w", err)
		}

		c := resp.Carbon
		var verifiedAt *time.Time
		if c.VerifiedAt != nil {
			t := time.Unix(c.VerifiedAt.Seconds, int64(c.VerifiedAt.Nanos))
			verifiedAt = &t
		}

		res := &models.CarbonFootprintResponse{
			BPAN:                 c.Bpan,
			TotalEmissionsKgCo2e: float32(c.TotalEmissionsKgCo2E),
			Verified:             c.Verified,
			VerifiedAt:           verifiedAt,
		}
		if c.VerifiedBy != "" {
			res.VerifiedBy = &c.VerifiedBy
		}
		if c.VerificationStandard != "" {
			res.VerificationStandard = &c.VerificationStandard
		}
		return res, nil
	}

	return &models.CarbonFootprintResponse{
		BPAN:                 bpan,
		TotalEmissionsKgCo2e: 0,
		Verified:             false,
	}, nil
}

// CompareCarbonFootprints compares two batteries' carbon footprints.
func (s *CarbonService) CompareCarbonFootprints(
	ctx context.Context,
	bpanA, bpanB string,
) (*models.CarbonComparison, error) {
	slog.Info("comparing carbon footprints", "bpan_a", bpanA, "bpan_b", bpanB)

	if s.carbonClient != nil {
		resp, err := s.carbonClient.CompareCarbon(ctx, &carbonv1.CompareCarbonRequest{
			BpanA: bpanA,
			BpanB: bpanB,
		})
		if err != nil {
			return nil, fmt.Errorf("gRPC error: %w", err)
		}

		c := resp.Comparison
		return &models.CarbonComparison{
			BpanA:      c.BpanA,
			BpanB:      c.BpanB,
			Stage1Delta: float32(c.RawMaterialDelta),
			Stage2Delta: float32(c.ManufacturingDelta),
			Stage3Delta: float32(c.TransportDelta),
			Stage4Delta: float32(c.UsageDelta),
			Stage5Delta: float32(c.RecyclingDelta),
			TotalDelta:  float32(c.TotalDeltaKgCo2E),
			BpanALower:  c.TotalDeltaKgCo2E < 0,
		}, nil
	}

	return &models.CarbonComparison{
		BpanA:     bpanA,
		BpanB:     bpanB,
		TotalDelta: 0,
	}, nil
}
