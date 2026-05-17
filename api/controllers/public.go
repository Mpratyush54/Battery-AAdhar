// public.go — Public battery lookup and QR validation (no authentication)

package controllers

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/Mpratyush54/Battery-AAdhar/api/services"
	"github.com/go-chi/chi/v5"
)

// GetPublicBattery — GET /api/v1/public/battery/{bpan}
// @Summary Get public battery information
// @Description Returns only public fields for a battery (no authentication required)
// @Tags public
// @Param bpan path string true "BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} map[string]interface{} "Public battery data"
// @Failure 400 {object} map[string]string "Bad request"
// @Failure 404 {object} map[string]string "Battery not found"
// @Router /api/v1/public/battery/{bpan} [get]
func GetPublicBattery(batteryService *services.BatteryService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")

		if bpan == "" {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "bpan required"})
			return
		}

		slog.Info("public battery lookup", "bpan", bpan)

		resp, err := batteryService.GetBatteryFull(r.Context(), bpan)
		if err != nil {
			slog.Warn("public battery lookup failed", "bpan", bpan, "error", err)
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "battery not found"})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"bpan":               resp.BPAN,
			"manufacturer":       resp.Manufacturer,
			"static_data":        resp.StaticData,
			"compliance_status":  resp.ComplianceStatus,
			"created_at":         resp.CreatedAt,
		})
	}
}

// QrValidationRequest is the request body for QR payload validation.
type QrValidationRequest struct {
	PayloadJSON string `json:"payload_json"`
}

// ValidateQRPayload — POST /api/v1/public/qr/validate
// @Summary Validate QR code payload
// @Description Validates a QR code payload for integrity (no authentication required)
// @Tags public
// @Accept json
// @Produce json
// @Param body body QrValidationRequest true "QR payload to validate"
// @Success 200 {object} map[string]interface{} "Validation result"
// @Failure 400 {object} map[string]string "Bad request"
// @Failure 500 {object} map[string]string "Validation failed"
// @Router /api/v1/public/qr/validate [post]
func ValidateQRPayload(qrService *services.QrService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req QrValidationRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		if req.PayloadJSON == "" {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "payload_json required"})
			return
		}

		slog.Info("QR payload validation requested")

		valid, err := qrService.ValidatePayload(r.Context(), req.PayloadJSON)
		if err != nil {
			slog.Warn("QR validation failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "validation failed"})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"valid": valid,
		})
	}
}

// RegisterPublicRoutes registers all public (unauthenticated) endpoints.
func RegisterPublicRoutes(r chi.Router, batteryService *services.BatteryService, qrService *services.QrService) {
	r.Get("/public/battery/{bpan}", GetPublicBattery(batteryService))
	r.Post("/public/qr/validate", ValidateQRPayload(qrService))
}
