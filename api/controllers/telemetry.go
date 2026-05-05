// telemetry.go — Battery telemetry (voltage, current, temperature, etc.)

package controllers

import (
	"github.com/go-chi/chi/v5"
	"net/http"
)

// GetTelemetry godoc
// @Summary      Get latest telemetry data
// @Description  Returns the most recent telemetry readings for a battery
// @Tags         telemetry
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {object}  map[string]interface{}
// @Failure      404  {object}  map[string]string  "Battery not found"
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/telemetry [get]
// @Security     BearerAuth
func GetTelemetry(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// UploadTelemetry godoc
// @Summary      Upload telemetry data
// @Description  Uploads new telemetry readings from BMS/OEM (encrypted at rest)
// @Tags         telemetry
// @Accept       json
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Param        body   body   object  true  "Telemetry data payload"
// @Success      200  {object}  map[string]interface{}
// @Failure      400  {object}  map[string]string
// @Failure      403  {object}  map[string]string  "Forbidden"
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/telemetry [post]
// @Security     BearerAuth
func UploadTelemetry(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetTelemetryHistory godoc
// @Summary      Get telemetry history
// @Description  Returns historical telemetry data for a battery
// @Tags         telemetry
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {array}   map[string]interface{}
// @Failure      404  {object}  map[string]string
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/telemetry/history [get]
// @Security     BearerAuth
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
