// public.go — Public battery lookup and QR validation (no authentication)

package controllers

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/go-chi/chi/v5"
)

// GetPublicBattery — GET /public/battery/{bpan}
// Returns only public fields: BPAN, chemistry, capacity, recyclable %, lifecycle state.
// No authentication required.
func GetPublicBattery(w http.ResponseWriter, r *http.Request) {
	bpan := chi.URLParam(r, "bpan")

	if bpan == "" {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(map[string]string{"error": "bpan required"})
		return
	}

	slog.Info("public battery lookup", "bpan", bpan)

	// TODO Day 16: Fetch from DB, return only public fields
	// For now, return mock public data
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"bpan":               bpan,
		"chemistry_type":     "NMC",
		"capacity_kwh":       30,
		"recyclable_percent": 87.5,
		"lifecycle_state":    "OPERATIONAL",
		"compliance_status":  "compliant",
	})
}

// QrValidationRequest is the request body for QR payload validation.
type QrValidationRequest struct {
	PayloadJSON string `json:"payload_json"`
}

// ValidateQRPayload — POST /public/qr/validate
// Validates a QR code payload for integrity.
func ValidateQRPayload(w http.ResponseWriter, r *http.Request) {
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

	// TODO Day 16: Call Rust gRPC service to validate QR payload
	// For now, return basic validation
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"valid": true,
		"note":  "QR payload validation will be wired to Rust core",
	})
}

// RegisterPublicRoutes registers all public (unauthenticated) endpoints.
func RegisterPublicRoutes(r chi.Router) {
	r.Get("/public/battery/{bpan}", GetPublicBattery)
	r.Post("/public/qr/validate", ValidateQRPayload)
}
