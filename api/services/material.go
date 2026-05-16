// material.go — Material composition service layer
// Orchestrates gRPC calls to the Rust core for BMCS operations.

package services

import (
	"context"
	"fmt"
	"log/slog"

	batteryv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/battery/v1"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

// MaterialService handles BMCS operations via gRPC to Rust core.
type MaterialService struct {
	batteryClient batteryv1.BatteryServiceClient
}

// NewMaterialService creates a new material service.
func NewMaterialService(cc *grpc.ClientConn) *MaterialService {
	if cc == nil {
		return &MaterialService{}
	}
	return &MaterialService{
		batteryClient: cc.BatteryClient,
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
	if req.CathodeMaterial == "" || req.AnodeMaterial == "" {
		return nil, fmt.Errorf("cathode_material and anode_material are required")
	}

	if s.batteryClient == nil {
		return nil, fmt.Errorf("material service: gRPC client not connected")
	}

	slog.Info("submitting BMCS",
		"bpan", bpan,
		"submitter_id", submitterID,
		"cathode", req.CathodeMaterial,
	)

	resp, err := s.batteryClient.SubmitMaterialComposition(ctx, &batteryv1.SubmitMaterialCompositionRequest{
		Bpan:       bpan,
		SubmitterId: submitterID,
		Composition: &batteryv1.MaterialComposition{
			Bpan:                bpan,
			CathodeMaterial:     req.CathodeMaterial,
			AnodeMaterial:       req.AnodeMaterial,
			ElectrolyteType:     req.ElectrolyteType,
			SeparatorMaterial:   req.SeparatorMaterial,
			RecyclablePercentage: req.RecyclablePercent,
			LithiumContentG:     req.LithiumContentG,
			CobaltContentG:      req.CobaltContentG,
			NickelContentG:      req.NickelContentG,
			ManganeseContentG:   req.ManganeseContentG,
			LeadContentG:        req.LeadContentG,
			CadmiumContentG:     req.CadmiumContentG,
			HazardousSubstances: req.HazardousSubstances,
			SupplyChainSource:   req.SupplyChainSource,
		},
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}

	return &models.SubmitMaterialResponse{
		Success:   resp.Success,
		DataHash:  resp.DataHash,
		EventHash: resp.EventHash,
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

	if s.batteryClient == nil {
		return nil, fmt.Errorf("material service: gRPC client not connected")
	}

	slog.Info("fetching BMCS",
		"bpan", bpan,
		"requester_role", requesterRole,
	)

	resp, err := s.batteryClient.GetMaterialComposition(ctx, &batteryv1.GetMaterialCompositionRequest{
		Bpan:          bpan,
		RequesterRole: requesterRole,
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}

	if resp.Composition == nil {
		return nil, fmt.Errorf("no BMCS data found for BPAN %s", bpan)
	}

	comp := resp.Composition
	return &models.MaterialCompositionResponse{
		BPAN:              comp.Bpan,
		CathodeMaterial:   comp.CathodeMaterial,
		AnodeMaterial:     comp.AnodeMaterial,
		ElectrolyteType:   comp.ElectrolyteType,
		SeparatorMaterial: comp.SeparatorMaterial,
		RecyclablePercent: comp.RecyclablePercentage,
		Partial:           resp.Partial,
	}, nil
}
