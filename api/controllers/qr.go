// qr.go — QR code generation and retrieval

package controllers

import (
	"encoding/json"
	"fmt"
	"net/http"

	qrpkg "github.com/Mpratyush54/Battery-AAdhar/api/qr"
	"github.com/go-chi/chi/v5"
)

// GetQRCode godoc
// @Summary      Generate QR code for a battery
// @Description  Generates a QR code PNG image containing the battery's BPAN payload
// @Tags         qr
// @Produce      image/png
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {file}    binary  "QR Code PNG image"
// @Failure      400  {object}  map[string]string  "Invalid BPAN"
// @Failure      500  {object}  map[string]string  "QR generation failed"
// @Router       /batteries/{bpan}/qr [get]
// @Security     BearerAuth
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

// ScanQRCode godoc
// @Summary      Scan and decode a QR code
// @Description  Decodes a QR code payload and returns battery information
// @Tags         qr
// @Accept       json
// @Produce      json
// @Param        body   body   object  true  "QR payload"
// @Success      200  {object}  map[string]interface{}
// @Failure      400  {object}  map[string]string
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/scan [post]
func ScanQRCode(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

func RegisterQRRoutes(r chi.Router) {
	r.Get("/batteries/{bpan}/qr", GetQRCode)
	r.Post("/batteries/scan", ScanQRCode)
}
