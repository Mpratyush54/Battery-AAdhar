// compliance_service.go — Compliance checking service layer
// Orchestrates gRPC calls to the Rust core for compliance operations.

package services

import (
	"context"
	"fmt"
	"log/slog"
	"time"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
	lifecyclev1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/lifecycle/v1"
)

// ComplianceService handles compliance checking and ZK verification.
type ComplianceService struct {
	lifecycleClient lifecyclev1.LifecycleServiceClient
}

// NewComplianceService creates a new compliance service.
func NewComplianceService() *ComplianceService {
	return &ComplianceService{}
}

// NewComplianceServiceWithClient creates a compliance service with gRPC client.
func NewComplianceServiceWithClient(cc *grpc.ClientConn) *ComplianceService {
	if cc == nil {
		return &ComplianceService{}
	}
	return &ComplianceService{
		lifecycleClient: cc.LifecycleClient,
	}
}

// GetComplianceStatus checks a single battery's compliance status via gRPC.
func (s *ComplianceService) GetComplianceStatus(
	ctx context.Context,
	bpan string,
) (*models.ComplianceStatusResponse, error) {
	if s.lifecycleClient == nil {
		return nil, fmt.Errorf("compliance service: gRPC client not connected")
	}

	resp, err := s.lifecycleClient.CheckCompliance(ctx, &lifecyclev1.CheckComplianceRequest{
		Bpan: bpan,
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}

	violations := make([]models.ComplianceViolation, len(resp.Violations))
	for i, v := range resp.Violations {
		var deadline *time.Time
		if v.ActionDeadline != nil {
			t := time.Unix(v.ActionDeadline.Seconds, int64(v.ActionDeadline.Nanos))
			deadline = &t
		}
		detectedAt := time.Unix(v.DetectedAt.Seconds, int64(v.DetectedAt.Nanos))

		violations[i] = models.ComplianceViolation{
			ViolationType:  v.ViolationType,
			Severity:       v.Severity,
			Description:    v.Description,
			RequiresAction: v.RequiresAction,
			ActionDeadline: deadline,
			DetectedAt:     detectedAt,
		}
	}

	lastChecked := time.Unix(resp.LastCheckedAt.Seconds, int64(resp.LastCheckedAt.Nanos))

	return &models.ComplianceStatusResponse{
		BPAN:          resp.Bpan,
		Status:        resp.Status,
		Violations:    violations,
		CriticalCount: resp.CriticalCount,
		WarningCount:  resp.WarningCount,
		LastCheckedAt: lastChecked,
	}, nil
}

// TriggerComplianceScan triggers a full compliance scan via gRPC.
func (s *ComplianceService) TriggerComplianceScan(ctx context.Context) (*models.ComplianceDashboard, error) {
	if s.lifecycleClient == nil {
		return nil, fmt.Errorf("compliance service: gRPC client not connected")
	}

	slog.Info("triggering compliance scan via gRPC")

	resp, err := s.lifecycleClient.ScanAllBatteries(ctx, &lifecyclev1.ScanAllBatteriesRequest{})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}

	return &models.ComplianceDashboard{
		TotalBatteries:          resp.TotalScanned,
		BatteriesWithViolations: resp.ViolationsFound,
		CriticalViolations:      0,
		WarningViolations:       0,
		ComplianceRate:          100.0,
	}, nil
}

// VerifyOperational generates a ZK proof that battery meets operational standards.
func (s *ComplianceService) VerifyOperational(
	ctx context.Context,
	bpan string,
	soh float32,
) (*models.ComplianceProofResponse, error) {
	if s.lifecycleClient == nil {
		return nil, fmt.Errorf("compliance service: gRPC client not connected")
	}

	resp, err := s.lifecycleClient.GenerateComplianceProof(ctx, &lifecyclev1.GenerateComplianceProofRequest{
		Bpan:       bpan,
		Requirement: "operational",
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}

	return &models.ComplianceProofResponse{
		BPAN:        resp.Bpan,
		Requirement: resp.Requirement,
		Statement:   resp.Statement,
		Proof:       resp.Proof,
		Commitment:  resp.Commitment,
	}, nil
}

// VerifySecondLife generates a ZK proof for second-life eligibility.
func (s *ComplianceService) VerifySecondLife(
	ctx context.Context,
	bpan string,
) (*models.ComplianceProofResponse, error) {
	if s.lifecycleClient == nil {
		return nil, fmt.Errorf("compliance service: gRPC client not connected")
	}

	resp, err := s.lifecycleClient.GenerateComplianceProof(ctx, &lifecyclev1.GenerateComplianceProofRequest{
		Bpan:       bpan,
		Requirement: "second_life",
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}

	return &models.ComplianceProofResponse{
		BPAN:        resp.Bpan,
		Requirement: resp.Requirement,
		Statement:   resp.Statement,
		Proof:       resp.Proof,
		Commitment:  resp.Commitment,
	}, nil
}

// GetViolations retrieves compliance violations for a battery.
func (s *ComplianceService) GetViolations(ctx context.Context, bpan string) ([]models.ComplianceViolation, error) {
	status, err := s.GetComplianceStatus(ctx, bpan)
	if err != nil {
		return nil, err
	}
	return status.Violations, nil
}

// GetComplianceDashboard retrieves aggregated compliance stats.
func (s *ComplianceService) GetComplianceDashboard(ctx context.Context) (*models.ComplianceDashboard, error) {
	slog.Info("fetching compliance dashboard")

	if s.lifecycleClient != nil {
		resp, err := s.lifecycleClient.ScanAllBatteries(ctx, &lifecyclev1.ScanAllBatteriesRequest{})
		if err != nil {
			return nil, fmt.Errorf("gRPC error: %w", err)
		}

		total := resp.TotalScanned
		violations := resp.ViolationsFound
		compliant := total - violations
		var rate float32
		if total > 0 {
			rate = float32(compliant) / float32(total) * 100.0
		} else {
			rate = 100.0
		}

		return &models.ComplianceDashboard{
			TotalBatteries:          total,
			BatteriesWithViolations: violations,
			CriticalViolations:      0,
			WarningViolations:       0,
			ComplianceRate:          float32(rate),
		}, nil
	}

	return &models.ComplianceDashboard{
		TotalBatteries:          0,
		BatteriesWithViolations: 0,
		CriticalViolations:      0,
		WarningViolations:       0,
		ComplianceRate:          100.0,
	}, nil
}
