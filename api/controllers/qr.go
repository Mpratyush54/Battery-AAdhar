// qr.go — QR code generation and retrieval

package controllers

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"

	"github.com/Mpratyush54/Battery-AAdhar/api/services"
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
func GetQRCode(qrService *services.QrService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpanStr := chi.URLParam(r, "bpan")

		pngBytes, err := qrService.GenerateQRCode(r.Context(), bpanStr)
		if err != nil {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		// Return PNG with metadata
		w.Header().Set("Content-Type", "image/png")
		w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=%s_qr.png", bpanStr))
		w.WriteHeader(http.StatusOK)
		w.Write(pngBytes)
	}
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
// @Failure      500  {object}  map[string]string  "Validation failed"
// @Router       /batteries/scan [post]
// @Security     BearerAuth
func ScanQRCode(qrService *services.QrService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req struct {
			PayloadJSON string `json:"payload_json"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		valid, err := qrService.ValidatePayload(r.Context(), req.PayloadJSON)
		if err != nil {
			slog.Warn("QR scan validation failed", "error", err)
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"valid": valid,
		})
	}
}

func RegisterQRRoutes(r chi.Router, qrService *services.QrService) {
	r.Get("/batteries/{bpan}/qr", GetQRCode(qrService))
	r.Post("/batteries/scan", ScanQRCode(qrService))
}
