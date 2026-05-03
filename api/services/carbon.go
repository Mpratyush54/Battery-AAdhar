// carbon.go — Carbon footprint service orchestration

package services

import (
	"context"
	"fmt"
	"log/slog"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

type CarbonService struct {
	encryptionService *EncryptionService
	// TODO Day 7: add repository
}

func NewCarbonService(encSvc *EncryptionService) *CarbonService {
	return &CarbonService{
		encryptionService: encSvc,
	}
}

func (s *CarbonService) SubmitCarbonFootprint(
	ctx context.Context,
	bpan string,
	req *models.CarbonFootprintRequest,
	requesterRole string,
) (string, error) {
	if requesterRole != "manufacturer" && requesterRole != "importer" && requesterRole != "admin" {
		return "", fmt.Errorf("only manufacturer can submit carbon data")
	}

	// TODO Day 7: Call Rust service to encrypt + store
	// For now, return submission ID
	submissionID := fmt.Sprintf("sub-%s-bcf", bpan)

	slog.Info("carbon footprint submitted",
		"bpan", bpan,
		"submission_id", submissionID,
		"total_emissions", req.RawMaterialEmissionsKgCo2e+req.ManufacturingEmissionsKgCo2e+req.TransportEmissionsKgCo2e+req.UsageEmissionsKgCo2e+req.RecyclingEmissionsKgCo2e,
	)

	return submissionID, nil
}

func (s *CarbonService) VerifyCarbonFootprint(
	ctx context.Context,
	bpan string,
	verifiedBy string,
	standard string,
	requesterRole string,
) error {
	if requesterRole != "verifier" && requesterRole != "regulator" && requesterRole != "admin" {
		return fmt.Errorf("only verifier can verify")
	}

	// TODO Day 7: Check hash integrity, mark verified in DB

	slog.Info("carbon verified",
		"bpan", bpan,
		"verified_by", verifiedBy,
		"standard", standard,
	)

	return nil
}

func (s *CarbonService) GetCarbonFootprint(
	ctx context.Context,
	bpan string,
	requesterRole string,
) (*models.CarbonFootprintResponse, error) {
	// TODO Day 7: Fetch from DB based on role

	return &models.CarbonFootprintResponse{
		BPAN:                 bpan,
		TotalEmissionsKgCo2e: 157.0,
		EmissionsPerKwh:      5.23,
		Verified:             true,
		VerifiedBy:           stringPtr("TUV-INDIA"),
	}, nil
}

func (s *CarbonService) CompareCarbonFootprints(
	ctx context.Context,
	bpanA string,
	bpanB string,
) (*models.CarbonComparison, error) {
	// TODO Day 7: Fetch both from DB, compute deltas

	return &models.CarbonComparison{
		BpanA:      bpanA,
		BpanB:      bpanB,
		TotalDelta: 10.0,
		BpanALower: true,
	}, nil
}

func stringPtr(s string) *string {
	return &s
}
