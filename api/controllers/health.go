// health.go — State of Health (SoH) tracking

package controllers

import (
	"github.com/go-chi/chi/v5"
	"net/http"
)

// GetStateOfHealth godoc
// @Summary      Get current State of Health
// @Description  Returns the latest SoH value and status for a battery
// @Tags         health
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {object}  map[string]interface{}
// @Failure      404  {object}  map[string]string  "Battery not found"
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/health [get]
// @Security     BearerAuth
func GetStateOfHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// UpdateStateOfHealth godoc
// @Summary      Update State of Health
// @Description  Records a new SoH reading for a battery (service provider only)
// @Tags         health
// @Accept       json
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Param        body   body   object  true  "SoH update payload"
// @Success      200  {object}  map[string]interface{}
// @Failure      400  {object}  map[string]string
// @Failure      403  {object}  map[string]string  "Forbidden"
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/health [patch]
// @Security     BearerAuth
func UpdateStateOfHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetHealthHistory godoc
// @Summary      Get SoH history
// @Description  Returns the full history of SoH readings for a battery
// @Tags         health
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {array}   map[string]interface{}
// @Failure      404  {object}  map[string]string
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/health/history [get]
// @Security     BearerAuth
func GetHealthHistory(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

func RegisterHealthRoutes(r chi.Router) {
	r.Get("/batteries/{bpan}/health", GetStateOfHealth)
	r.Patch("/batteries/{bpan}/health", UpdateStateOfHealth)
	r.Get("/batteries/{bpan}/health/history", GetHealthHistory)
}
