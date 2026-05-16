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

// CheckCompliance checks a single battery's compliance status.
func (s *ComplianceService) CheckCompliance(
	ctx context.Context,
	bpan string,
	soh float32,
	hasMaterial bool,
	hasCarbon bool,
) (*models.ComplianceStatusResponse, error) {
	if bpan == "" {
		return nil, fmt.Errorf("bpan is required")
	}

	slog.Info("checking compliance",
		"bpan", bpan,
		"soh", soh,
		"has_material", hasMaterial,
		"has_carbon", hasCarbon,
	)

	var violations []models.ComplianceViolation
	status := "COMPLIANT"

	if soh < 30 {
		violations = append(violations, models.ComplianceViolation{
			ViolationType:  "END_OF_LIFE",
			Severity:       "CRITICAL",
			Description:    fmt.Sprintf("Battery SoH %.1f%% < 30%%, end-of-life recycling required", soh),
			RequiresAction: true,
			DetectedAt:     time.Now(),
		})
		status = "VIOLATIONS_EXIST"
	} else if soh < 80 {
		violations = append(violations, models.ComplianceViolation{
			ViolationType:  "SECOND_LIFE_ELIGIBLE",
			Severity:       "INFO",
			Description:    fmt.Sprintf("Battery SoH %.1f%% eligible for second-life", soh),
			RequiresAction: false,
			DetectedAt:     time.Now(),
		})
		status = "WARNINGS_EXIST"
	}

	if !hasMaterial {
		violations = append(violations, models.ComplianceViolation{
			ViolationType:  "MISSING_BMCS",
			Severity:       "CRITICAL",
			Description:    "Material Composition (BMCS) not submitted",
			RequiresAction: true,
			DetectedAt:     time.Now(),
		})
		status = "VIOLATIONS_EXIST"
	}

	_ = hasCarbon // Used in future compliance rules

	return &models.ComplianceStatusResponse{
		BPAN:       bpan,
		Status:     status,
		Violations: violations,
	}, nil
}

// TriggerComplianceScan triggers a full compliance scan.
func (s *ComplianceService) TriggerComplianceScan(ctx context.Context) (*models.ComplianceDashboard, error) {
	slog.Info("triggering compliance scan")

	// TODO: Wire to Rust gRPC for full scan
	return &models.ComplianceDashboard{
		TotalBatteries:          0,
		BatteriesWithViolations: 0,
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

	resp, err := s.lifecycleClient.VerifyOperational(ctx, &lifecyclev1.VerifyOperationalRequest{
		Bpan: bpan,
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}

	return &models.ComplianceProofResponse{
		BPAN:       bpan,
		Requirement: "operational",
		Statement:  fmt.Sprintf("Battery SoH > 80%% (actual: %.1f%%)", soh),
		Proof:      resp.ZkProof,
		Commitment: resp.PublicInputs,
	}, nil
}

// GetViolations retrieves compliance violations for a battery.
func (s *ComplianceService) GetViolations(ctx context.Context, bpan string) ([]models.ComplianceViolation, error) {
	slog.Info("fetching violations", "bpan", bpan)

	// TODO: Wire to Rust gRPC
	return []models.ComplianceViolation{}, nil
}

// GetComplianceDashboard retrieves aggregated compliance stats.
func (s *ComplianceService) GetComplianceDashboard(ctx context.Context) (*models.ComplianceDashboard, error) {
	slog.Info("fetching compliance dashboard")

	// TODO: Wire to Rust gRPC
	return &models.ComplianceDashboard{
		TotalBatteries:          0,
		BatteriesWithViolations: 0,
		CriticalViolations:      0,
		WarningViolations:       0,
		ComplianceRate:          100.0,
	}, nil
}
