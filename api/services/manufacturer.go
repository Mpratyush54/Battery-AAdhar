package services

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/google/uuid"
)

// ManufacturerService handles manufacturer profile management and batch registration.
type ManufacturerService struct {
	// In production, this would use gRPC to the Rust core engine.
	// For now, the controller returns service-unavailable if nil.
}

// NewManufacturerService creates a new manufacturer service.
func NewManufacturerService() *ManufacturerService {
	return &ManufacturerService{}
}

// RegisterManufacturer registers a new manufacturer with regulator-assigned 3-char code.
func (s *ManufacturerService) RegisterManufacturer(
	ctx context.Context,
	req *models.RegisterManufacturerRequest,
	regulatorID string,
) (*models.RegisterManufacturerResponse, error) {
	slog.Info("register_manufacturer",
		"name", req.Name,
		"country", req.CountryCode,
		"regulator_id", regulatorID,
	)

	id := uuid.New().String()
	code := s.assignCode(req.Name)

	resp := &models.RegisterManufacturerResponse{
		ID:               id,
		ManufacturerCode: code,
		Name:             req.Name,
	}

	slog.Info("manufacturer_registered",
		"id", id,
		"code", code,
		"name", req.Name,
	)

	return resp, nil
}

// ListManufacturers returns all registered manufacturers.
func (s *ManufacturerService) ListManufacturers(ctx context.Context) ([]models.ManufacturerProfile, error) {
	slog.Info("list_manufacturers")
	return []models.ManufacturerProfile{}, nil
}

// BatchRegisterBatteries submits a batch of battery registrations.
func (s *ManufacturerService) BatchRegisterBatteries(
	ctx context.Context,
	req *models.BatchBatteryRequest,
	actorID string,
) (*models.BatchBatteryResponse, error) {
	slog.Info("batch_register_batteries",
		"mfr_code", req.ManufacturerCode,
		"count", len(req.Batteries),
		"actor_id", actorID,
	)

	if len(req.Batteries) == 0 {
		return nil, fmt.Errorf("batch must contain at least 1 battery")
	}

	results := make([]models.BatteryBatchResult, 0, len(req.Batteries))
	for _, row := range req.Batteries {
		bpan := fmt.Sprintf("%s%02d%06d", req.ManufacturerCode, row.ProductionYear%100, len(results)+1)
		results = append(results, models.BatteryBatchResult{
			BPAN:       bpan,
			StaticHash: "batch_computed_hash",
			Status:     "PENDING",
		})
	}

	auditID := uuid.New().String()

	resp := &models.BatchBatteryResponse{
		ManufacturerID: actorID,
		Total:          len(results),
		Batteries:      results,
		AuditID:        auditID,
	}

	slog.Info("batch_registered",
		"total", len(results),
		"audit_id", auditID,
	)

	return resp, nil
}

// ListOwnBatteries returns paginated list of batteries for a manufacturer.
func (s *ManufacturerService) ListOwnBatteries(
	ctx context.Context,
	manufacturerID string,
	limit int64,
	offset int64,
) ([]models.ManufacturerBatterySummary, error) {
	slog.Info("list_own_batteries",
		"manufacturer_id", manufacturerID,
		"limit", limit,
		"offset", offset,
	)
	return []models.ManufacturerBatterySummary{}, nil
}

// GetDashboard returns dashboard aggregates for a manufacturer.
func (s *ManufacturerService) GetDashboard(
	ctx context.Context,
	manufacturerID string,
) (*models.ManufacturerDashboard, error) {
	slog.Info("manufacturer_dashboard",
		"manufacturer_id", manufacturerID,
	)

	dashboard := &models.ManufacturerDashboard{
		TotalBatteries:       0,
		Operational:          0,
		PendingRegistrations: 0,
		RejectedRegistrations: 0,
		SecondLife:           0,
		EndOfLife:            0,
		AverageSoH:           0.0,
		ComplianceViolations: 0,
	}

	return dashboard, nil
}

// SubmitMaterial submits BMCS data for a battery.
func (s *ManufacturerService) SubmitMaterial(
	ctx context.Context,
	bpan string,
	req *models.MaterialCompositionRequest,
) error {
	slog.Info("submit_material", "bpan", bpan)
	return nil
}

// SubmitCarbon submits BCF data for a battery.
func (s *ManufacturerService) SubmitCarbon(
	ctx context.Context,
	bpan string,
	req *models.CarbonFootprintRequest,
) error {
	slog.Info("submit_carbon", "bpan", bpan)
	return nil
}

// assignCode generates a deterministic 3-char code from the manufacturer name.
// In production, this is replaced by the Rust service via gRPC.
func (s *ManufacturerService) assignCode(name string) string {
	if len(name) == 0 {
		return "AAA"
	}

	// Use first 3 chars, uppercased
	code := ""
	for _, c := range name {
		if c >= 'a' && c <= 'z' {
			code += string(c - 32)
		} else if c >= 'A' && c <= 'Z' {
			code += string(c)
		}
		if len(code) == 3 {
			return code
		}
	}

	// Pad if too short
	for len(code) < 3 {
		code += "X"
	}

	return code
}

// Ensure ManufacturerService implements the interface
var _ interface {
	RegisterManufacturer(context.Context, *models.RegisterManufacturerRequest, string) (*models.RegisterManufacturerResponse, error)
	ListManufacturers(context.Context) ([]models.ManufacturerProfile, error)
	BatchRegisterBatteries(context.Context, *models.BatchBatteryRequest, string) (*models.BatchBatteryResponse, error)
	ListOwnBatteries(context.Context, string, int64, int64) ([]models.ManufacturerBatterySummary, error)
	GetDashboard(context.Context, string) (*models.ManufacturerDashboard, error)
	SubmitMaterial(context.Context, string, *models.MaterialCompositionRequest) error
	SubmitCarbon(context.Context, string, *models.CarbonFootprintRequest) error
} = (*ManufacturerService)(nil)

// Avoid unused import error for json
var _ = json.Marshal
var _ = fmt.Sprintf
