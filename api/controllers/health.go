// health.go — State of Health (SoH) tracking

package controllers

import (
	"github.com/go-chi/chi/v5"
	"net/http"
)

// GetStateOfHealth — GET /api/v1/batteries/{bpan}/health
func GetStateOfHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// UpdateStateOfHealth — PATCH /api/v1/batteries/{bpan}/health (service provider)
func UpdateStateOfHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetHealthHistory — GET /api/v1/batteries/{bpan}/health/history
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
