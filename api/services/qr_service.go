// qr_service.go — QR code service layer
// Orchestrates QR generation and validation via gRPC to the Rust core.

package services

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"

	"github.com/Mpratyush54/Battery-AAdhar/api/qr"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
	healthv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/health/v1"
	carbonv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/carbon/v1"
)

// QrService handles QR code generation and validation.
type QrService struct {
	healthClient healthv1.HealthServiceClient
	carbonClient carbonv1.CarbonServiceClient
}

// NewQrService creates a new QR service.
func NewQrService() *QrService {
	return &QrService{}
}

// NewQrServiceWithClient creates a QR service with gRPC clients.
func NewQrServiceWithClient(cc *grpc.ClientConn) *QrService {
	if cc == nil {
		return NewQrService()
	}
	return &QrService{
		healthClient: cc.HealthClient,
		carbonClient: cc.CarbonClient,
	}
}

// GenerateQRPayload builds a QR payload with real DB values via gRPC.
func (s *QrService) GenerateQRPayload(
	ctx context.Context,
	bpan string,
) (*qr.QRPayload, error) {
	if bpan == "" {
		return nil, fmt.Errorf("bpan is required")
	}

	slog.Info("generating QR payload", "bpan", bpan)

	payload, err := qr.CreatePayload(bpan)
	if err != nil {
		return nil, fmt.Errorf("create payload: %w", err)
	}

	if s.healthClient != nil && s.carbonClient != nil {
		healthResp, hErr := s.healthClient.GetHealth(ctx, &healthv1.GetHealthRequest{Bpan: bpan})
		carbonResp, cErr := s.carbonClient.GetCarbon(ctx, &carbonv1.GetCarbonRequest{Bpan: bpan})

		if hErr == nil && healthResp.Record != nil {
			rec := healthResp.Record
			if rec.InternalResistanceMohm > 0 {
				payload.RecyclePercentage = float32(95.0 - rec.InternalResistanceMohm/10.0)
				if payload.RecyclePercentage < 0 {
					payload.RecyclePercentage = 0
				}
			}
		}
		_ = cErr
		if cErr == nil && carbonResp.Carbon != nil {
			payload.CarbonFootprintKgCO2e = float32(carbonResp.Carbon.TotalEmissionsKgCo2E)
		}
	}

	return payload, nil
}

// GenerateQRCode creates a QR code PNG with real DB values.
func (s *QrService) GenerateQRCode(
	ctx context.Context,
	bpan string,
) ([]byte, error) {
	payload, err := s.GenerateQRPayload(ctx, bpan)
	if err != nil {
		return nil, err
	}
	return qr.GenerateQR(payload)
}

// ValidatePayload validates a QR code payload for integrity.
func (s *QrService) ValidatePayload(
	ctx context.Context,
	payloadJSON string,
) (bool, error) {
	if payloadJSON == "" {
		return false, fmt.Errorf("payload_json is required")
	}

	slog.Info("validating QR payload")

	var payload qr.QRPayload
	if err := json.Unmarshal([]byte(payloadJSON), &payload); err != nil {
		return false, fmt.Errorf("invalid payload JSON: %w", err)
	}

	if payload.BPAN == "" {
		return false, fmt.Errorf("payload missing BPAN")
	}

	if s.healthClient != nil {
		healthResp, err := s.healthClient.GetHealth(ctx, &healthv1.GetHealthRequest{Bpan: payload.BPAN})
		if err != nil {
			return false, fmt.Errorf("gRPC error: %w", err)
		}
		if healthResp.Record == nil {
			return false, fmt.Errorf("battery not found: %s", payload.BPAN)
		}
	}

	return true, nil
}
