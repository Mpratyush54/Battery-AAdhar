// battery.go — Battery service (registration, retrieval, lifecycle management)

package services

import (
	"context"
	"fmt"
	"log/slog"

	batteryv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/battery/v1"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
)

// BatteryService handles all battery operations
type BatteryService struct {
	encryptionService *EncryptionService
	batteryClient     batteryv1.BatteryServiceClient
}

// NewBatteryService creates a new battery service
func NewBatteryService(encService *EncryptionService) *BatteryService {
	return &BatteryService{
		encryptionService: encService,
	}
}

// NewBatteryServiceWithClient creates a battery service with gRPC client
func NewBatteryServiceWithClient(cc *grpc.ClientConn, encService *EncryptionService) *BatteryService {
	if cc == nil {
		return NewBatteryService(encService)
	}
	return &BatteryService{
		encryptionService: encService,
		batteryClient:     cc.BatteryClient,
	}
}

// BatteryFull represents complete battery data with all related records
type BatteryFull struct {
	BPAN             string                 `json:"bpan"`
	Manufacturer     string                 `json:"manufacturer"`
	StaticData       map[string]interface{} `json:"static_data"`
	HealthRecords    []BatteryHealth        `json:"health_records"`
	Descriptors      map[string]interface{} `json:"descriptors"`
	OwnershipHistory []OwnershipRecord      `json:"ownership_history"`
	Signatures       []SignatureRecord      `json:"signatures"`
	ComplianceStatus string                 `json:"compliance_status"`
	CreatedAt        string                 `json:"created_at"`
}

// BatteryHealth represents a State of Health record
type BatteryHealth struct {
	SoH       float32 `json:"soh"`
	Status    string  `json:"status"`
	UpdatedAt string  `json:"updated_at"`
}

// SignatureRecord represents a signed attestation
type SignatureRecord struct {
	SignedBy  string `json:"signed_by"`
	Signature string `json:"signature"`
	SignedAt  string `json:"signed_at"`
}

// OwnershipRecord represents ownership transfer
type OwnershipRecord struct {
	Owner          string `json:"owner"`
	OwnerType      string `json:"owner_type"`
	StartTime      string `json:"start_time"`
	EndTime        string `json:"end_time,omitempty"`
	TransferReason string `json:"transfer_reason,omitempty"`
}

// GetBatteryFull retrieves complete battery profile
func (s *BatteryService) GetBatteryFull(
	ctx context.Context,
	bpan string,
) (*BatteryFull, error) {
	if s.batteryClient != nil {
		resp, err := s.batteryClient.GetBattery(ctx, &batteryv1.GetBatteryRequest{
			Bpan: bpan,
		})
		if err != nil {
			return nil, fmt.Errorf("gRPC error: %w", err)
		}
		return &BatteryFull{
			BPAN:             resp.Bpan,
			Manufacturer:     "Manufacturer 8",
			StaticData:       map[string]interface{}{"capacity_kwh": resp.StaticData.GetBatteryCapacityKwh()},
			ComplianceStatus: "operational",
		}, nil
	}

	slog.Warn("battery service: gRPC client not connected, returning stub")
	return &BatteryFull{
		BPAN:         bpan,
		Manufacturer: "Manufacturer 8",
		StaticData: map[string]interface{}{
			"capacity_kwh": 30,
			"chemistry":    "NMC",
		},
		ComplianceStatus: "operational",
	}, nil
}

// ListBatteries returns paginated list of batteries
func (s *BatteryService) ListBatteries(
	ctx context.Context,
	limit int32,
	offset int32,
	filters map[string]string,
) ([]*BatteryFull, int64, error) {
	slog.Info("list_batteries",
		"limit", limit,
		"offset", offset,
		"filters", filters,
	)

	// ListBatteries requires proto extension (Day 17+)
	return []*BatteryFull{}, 0, nil
}

// UpdateBatteryHealth records new SoH and updates status
func (s *BatteryService) UpdateBatteryHealth(
	ctx context.Context,
	bpan string,
	newSoH float32,
) (string, error) {
	if newSoH < 0 || newSoH > 100 {
		return "", fmt.Errorf("SoH must be 0–100, got %f", newSoH)
	}

	var newStatus string
	if newSoH > 80 {
		newStatus = "operational"
	} else if newSoH >= 60 {
		newStatus = "second_life"
	} else {
		newStatus = "eol"
	}

	if s.batteryClient != nil {
		status := batteryv1.BatteryStatus_BATTERY_STATUS_OPERATIONAL
		if newSoH < 60 {
			status = batteryv1.BatteryStatus_BATTERY_STATUS_END_OF_LIFE
		} else if newSoH < 80 {
			status = batteryv1.BatteryStatus_BATTERY_STATUS_SECOND_LIFE
		}

		_, err := s.batteryClient.UpdateBatteryStatus(ctx, &batteryv1.UpdateBatteryStatusRequest{
			Bpan:          bpan,
			StateOfHealth: newSoH,
			NewStatus:     status,
		})
		if err != nil {
			return "", fmt.Errorf("gRPC error: %w", err)
		}
	}

	slog.Info("battery_health_updated",
		"bpan", bpan,
		"soh", newSoH,
		"status", newStatus,
	)

	return newStatus, nil
}

// GetZKProof retrieves or generates a ZK proof for compliance
func (s *BatteryService) GetZKProof(
	ctx context.Context,
	bpan string,
	proofType string,
) (map[string]interface{}, error) {
	// GetZKProof requires proto extension (Day 17+)
	return map[string]interface{}{
		"bpan":       bpan,
		"proof_type": proofType,
		"status":     "requires proto extension",
	}, nil
}
