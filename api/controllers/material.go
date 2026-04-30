// material.go — Material composition & carbon footprint handlers (Day 8 BMCS)

package controllers

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
)

var materialService *services.MaterialService

func init() {
	materialService = services.NewMaterialService(nil)
}

// SubmitMaterialComposition — POST /api/v1/batteries/{bpan}/material
// @Summary Submit BMCS data for a battery
// @Tags Material
// @Accept json
// @Produce json
// @Param bpan path string true "Battery PAN"
// @Param body body models.MaterialCompositionRequest true "Material data"
// @Success 200 {object} models.SubmitMaterialResponse
// @Failure 400 {object} map[string]string
// @Failure 403 {object} map[string]string
// @Router /batteries/{bpan}/material [post]
func SubmitMaterialComposition(w http.ResponseWriter, r *http.Request) {
	bpan := chi.URLParam(r, "bpan")
	if bpan == "" {
		http.Error(w, `{"error":"bpan is required"}`, http.StatusBadRequest)
		return
	}

	// Get submitter identity from JWT claims
	claims := middleware.ClaimsFromContext(r.Context())
	submitterID := "00000000-0000-0000-0000-000000000000"
	if claims != nil {
		submitterID = claims.Subject
	}

	var req models.MaterialCompositionRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(map[string]string{"error": "invalid JSON: " + err.Error()})
		return
	}

	resp, err := materialService.SubmitMaterialComposition(r.Context(), bpan, submitterID, &req)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

// GetMaterialComposition — GET /api/v1/batteries/{bpan}/material
// @Summary Get BMCS data for a battery (role-gated)
// @Tags Material
// @Produce json
// @Param bpan path string true "Battery PAN"
// @Success 200 {object} models.MaterialCompositionResponse
// @Failure 404 {object} map[string]string
// @Router /batteries/{bpan}/material [get]
func GetMaterialComposition(w http.ResponseWriter, r *http.Request) {
	bpan := chi.URLParam(r, "bpan")
	if bpan == "" {
		http.Error(w, `{"error":"bpan is required"}`, http.StatusBadRequest)
		return
	}

	// Determine requester role from JWT claims
	requesterRole := "public"
	claims := middleware.ClaimsFromContext(r.Context())
	if claims != nil {
		requesterRole = claims.Role
	}

	resp, err := materialService.GetMaterialComposition(r.Context(), bpan, requesterRole)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
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

// RegisterMaterialRoutes registers all material/carbon endpoints with RBAC
func RegisterMaterialRoutes(r chi.Router) {
	// POST: only manufacturer/importer can submit
	r.With(middleware.HasAnyRole("manufacturer", "importer", "admin")).
		Post("/batteries/{bpan}/material", SubmitMaterialComposition)

	// GET: all authenticated roles can read (Rust layer controls field visibility)
	r.Get("/batteries/{bpan}/material", GetMaterialComposition)

	// Carbon footprint (stub)
	r.Get("/batteries/{bpan}/carbon", GetCarbonFootprint)

	// PATCH: manufacturer/admin only
	r.With(middleware.HasAnyRole("manufacturer", "admin")).
		Patch("/batteries/{bpan}/material", UpdateMaterialComposition)
}
