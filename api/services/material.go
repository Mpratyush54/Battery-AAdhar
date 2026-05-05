// material.go — Material composition service layer
// Orchestrates gRPC calls to the Rust core for BMCS operations.

package services

import (
	"context"
	"fmt"
	"log/slog"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

// MaterialService handles BMCS operations via gRPC to Rust core.
type MaterialService struct {
	encryptionService *EncryptionService
	// TODO: Add gRPC client once proto is regenerated
}

// NewMaterialService creates a new material service.
func NewMaterialService(encSvc *EncryptionService) *MaterialService {
	return &MaterialService{
		encryptionService: encSvc,
	}
}

// SubmitMaterialComposition sends BMCS data to Rust core for encryption+storage.
func (s *MaterialService) SubmitMaterialComposition(
	ctx context.Context,
	bpan string,
	submitterID string,
	req *models.MaterialCompositionRequest,
) (*models.SubmitMaterialResponse, error) {
	if bpan == "" {
		return nil, fmt.Errorf("bpan is required")
	}
	if submitterID == "" {
		return nil, fmt.Errorf("submitter_id is required")
	}

	// Validate required fields
	if req.CathodeMaterial == "" || req.AnodeMaterial == "" {
		return nil, fmt.Errorf("cathode_material and anode_material are required")
	}

	slog.Info("submitting BMCS",
		"bpan", bpan,
		"submitter_id", submitterID,
		"cathode", req.CathodeMaterial,
	)

	// TODO Day 8b: Call Rust gRPC SubmitMaterialComposition
	// For now, return a success stub with a hash placeholder
	return &models.SubmitMaterialResponse{
		Success:   true,
		DataHash:  fmt.Sprintf("sha256:bmcs:%s:pending", bpan),
		EventHash: fmt.Sprintf("sha256:event:%s:pending", bpan),
	}, nil
}

// GetMaterialComposition retrieves BMCS data, respecting role-based access.
func (s *MaterialService) GetMaterialComposition(
	ctx context.Context,
	bpan string,
	requesterRole string,
) (*models.MaterialCompositionResponse, error) {
	if bpan == "" {
		return nil, fmt.Errorf("bpan is required")
	}

	slog.Info("fetching BMCS",
		"bpan", bpan,
		"requester_role", requesterRole,
	)

	// TODO Day 8b: Call Rust gRPC GetMaterialComposition
	// For now, return a stub with public fields only
	return &models.MaterialCompositionResponse{
		BPAN:              bpan,
		CathodeMaterial:   "NMC811",
		AnodeMaterial:     "Graphite",
		ElectrolyteType:   "LiPF6",
		SeparatorMaterial: "PE/PP",
		RecyclablePercent: 92.5,
		Partial:           true,
	}, nil
}
