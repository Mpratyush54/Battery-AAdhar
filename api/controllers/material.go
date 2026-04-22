// material.go — Material composition & carbon footprint handlers

package controllers

import (
	"net/http"
	"github.com/go-chi/chi/v5"
)

// GetMaterialComposition — GET /api/v1/batteries/{bpan}/material
func GetMaterialComposition(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetCarbonFootprint — GET /api/v1/batteries/{bpan}/carbon
func GetCarbonFootprint(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// UpdateMaterialComposition — PATCH /api/v1/batteries/{bpan}/material (admin/manufacturer)
func UpdateMaterialComposition(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// RegisterRoutes registers all material/carbon endpoints
func RegisterMaterialRoutes(r chi.Router) {
	r.Get("/batteries/{bpan}/material", GetMaterialComposition)
	r.Get("/batteries/{bpan}/carbon", GetCarbonFootprint)
	r.Patch("/batteries/{bpan}/material", UpdateMaterialComposition)
}
