// qr.go — QR code generation and retrieval

package controllers

import (
	"encoding/json"
	"fmt"
	"net/http"

	qrpkg "github.com/Mpratyush54/Battery-AAdhar/api/qr"
	"github.com/go-chi/chi/v5"
)

// GetQRCode — GET /api/v1/batteries/{bpan}/qr
// Returns the QR code as PNG + metadata
func GetQRCode(w http.ResponseWriter, r *http.Request) {
	bpanStr := chi.URLParam(r, "bpan")

	// Create QR payload
	payload, err := qrpkg.CreatePayload(bpanStr)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
		return
	}

	// Generate QR PNG
	pngBytes, err := qrpkg.GenerateQR(payload)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(map[string]string{"error": fmt.Sprintf("QR generation failed: %v", err)})
		return
	}

	// Return PNG with metadata
	w.Header().Set("Content-Type", "image/png")
	w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=%s_qr.png", bpanStr))
	w.WriteHeader(http.StatusOK)
	w.Write(pngBytes)
}

// ScanQRCode — POST /api/v1/batteries/scan (upload QR payload)
func ScanQRCode(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

func RegisterQRRoutes(r chi.Router) {
	r.Get("/batteries/{bpan}/qr", GetQRCode)
	r.Post("/batteries/scan", ScanQRCode)
}
