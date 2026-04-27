// telemetry.go — Battery telemetry (voltage, current, temperature, etc.)

package controllers

import (
	"github.com/go-chi/chi/v5"
	"net/http"
)

// GetTelemetry — GET /api/v1/batteries/{bpan}/telemetry
func GetTelemetry(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// UploadTelemetry — POST /api/v1/batteries/{bpan}/telemetry (BMS/OEM)
func UploadTelemetry(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetTelemetryHistory — GET /api/v1/batteries/{bpan}/telemetry/history
func GetTelemetryHistory(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

func RegisterTelemetryRoutes(r chi.Router) {
	r.Get("/batteries/{bpan}/telemetry", GetTelemetry)
	r.Post("/batteries/{bpan}/telemetry", UploadTelemetry)
	r.Get("/batteries/{bpan}/telemetry/history", GetTelemetryHistory)
}
