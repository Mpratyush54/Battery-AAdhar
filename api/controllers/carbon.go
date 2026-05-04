// carbon.go — Carbon Footprint (BCF) HTTP endpoints
//
// POST /api/v1/batteries/{bpan}/carbon — submit BCF (manufacturer only)
// GET /api/v1/batteries/{bpan}/carbon — retrieve (all roles)
// POST /api/v1/batteries/{bpan}/carbon/verify — verify (verifier only)

package controllers

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
)

// SubmitCarbonFootprint — POST /api/v1/batteries/{bpan}/carbon
// @Summary Submit battery carbon footprint (5-stage emissions)
// @Description Submit BCF with raw material, manufacturing, transport, usage, recycling emissions
// @Tags carbon
// @Param bpan path string true "BPAN"
// @Param body body models.CarbonFootprintRequest true "Carbon data (5 stages)"
// @Accept json
// @Produce json
// @Success 201 {object} map[string]string "Submission ID"
// @Failure 403 {object} map[string]string "Unauthorized (non-manufacturer)"
// @Router /api/v1/batteries/{bpan}/carbon [post]
// @Security Bearer
func SubmitCarbonFootprint(carbonService *services.CarbonService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")
		claims := middleware.ClaimsFromContext(r.Context())

		// Only manufacturer can submit
		if claims.Role != "manufacturer" && claims.Role != "importer" && claims.Role != "admin" {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusForbidden)
			json.NewEncoder(w).Encode(map[string]string{
				"error": "only manufacturer can submit carbon footprint",
			})
			return
		}

		var req models.CarbonFootprintRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		// Call carbon service
		submissionID, err := carbonService.SubmitCarbonFootprint(r.Context(), bpan, &req, claims.Role)
		if err != nil {
			slog.Error("submit carbon failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "submission failed"})
			return
		}

		slog.Info("carbon footprint submitted",
			"bpan", bpan,
			"submission_id", submissionID,
			"submitted_by", claims.Subject,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]interface{}{
			"submission_id": submissionID,
			"bpan":          bpan,
			"total_emissions_kg_co2e": req.RawMaterialEmissionsKgCo2e + req.ManufacturingEmissionsKgCo2e + req.TransportEmissionsKgCo2e + req.UsageEmissionsKgCo2e + req.RecyclingEmissionsKgCo2e,
		})
	}
}

// GetCarbonFootprint — GET /api/v1/batteries/{bpan}/carbon
// @Summary Get battery carbon footprint
// @Description Retrieve BCF (all roles can view verified data)
// @Tags carbon
// @Param bpan path string true "BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} models.CarbonFootprintResponse "Carbon footprint"
// @Router /api/v1/batteries/{bpan}/carbon [get]
func GetCarbonFootprint(carbonService *services.CarbonService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")
		claims := middleware.ClaimsFromContext(r.Context())

		cf, err := carbonService.GetCarbonFootprint(r.Context(), bpan, claims.Role)
		if err != nil {
			slog.Error("get carbon failed", "error", err)
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "not found"})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(cf)
	}
}

// VerifyCarbonFootprint — POST /api/v1/batteries/{bpan}/carbon/verify
// @Summary Verify carbon footprint (third-party verifier only)
// @Description Mark carbon data as verified against standard (ISO 14040, PEF, EU ETS)
// @Tags carbon
// @Param bpan path string true "BPAN"
// @Param body body models.VerificationRequest true "Verification details"
// @Accept json
// @Produce json
// @Success 200 {object} map[string]string "Verified"
// @Failure 403 {object} map[string]string "Unauthorized (non-verifier)"
// @Router /api/v1/batteries/{bpan}/carbon/verify [post]
// @Security Bearer
func VerifyCarbonFootprint(carbonService *services.CarbonService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")
		claims := middleware.ClaimsFromContext(r.Context())

		// Only verifier can mark as verified
		if claims.Role != "verifier" && claims.Role != "regulator" && claims.Role != "admin" {
			w.WriteHeader(http.StatusForbidden)
			json.NewEncoder(w).Encode(map[string]string{
				"error": "only verifier can verify carbon data",
			})
			return
		}

		var req models.VerificationRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		// Verify
		err := carbonService.VerifyCarbonFootprint(r.Context(), bpan, claims.Subject, req.Standard, claims.Role)
		if err != nil {
			slog.Error("carbon verification failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "verification failed"})
			return
		}

		slog.Info("carbon footprint verified",
			"bpan", bpan,
			"verified_by", claims.Subject,
			"standard", req.Standard,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(map[string]string{
			"verified":  "true",
			"verified_by": claims.Subject,
			"standard": req.Standard,
		})
	}
}

// CompareCarbonFootprints — GET /api/v1/batteries/{bpan_a}/carbon/compare/{bpan_b}
// @Summary Compare carbon footprints (A vs B)
// @Description Emission delta per stage, identify lower-emission battery
// @Tags carbon
// @Param bpan_a path string true "First BPAN"
// @Param bpan_b path string true "Second BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} models.CarbonComparison "Comparison result"
// @Router /api/v1/batteries/{bpan_a}/carbon/compare/{bpan_b} [get]
func CompareCarbonFootprints(carbonService *services.CarbonService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan_a := chi.URLParam(r, "bpan_a")
		bpan_b := chi.URLParam(r, "bpan_b")

		comparison, err := carbonService.CompareCarbonFootprints(r.Context(), bpan_a, bpan_b)
		if err != nil {
			slog.Error("comparison failed", "error", err)
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "comparison failed"})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(comparison)
	}
}

var carbonService *services.CarbonService

func init() {
	carbonService = services.NewCarbonService(nil)
}

// RegisterCarbonRoutes registers all carbon-related routes
func RegisterCarbonRoutes(r chi.Router) {
	r.Post("/batteries/{bpan}/carbon", SubmitCarbonFootprint(carbonService))
	r.Get("/batteries/{bpan}/carbon", GetCarbonFootprint(carbonService))
	r.Post("/batteries/{bpan}/carbon/verify", VerifyCarbonFootprint(carbonService))
	r.Get("/batteries/{bpan}/carbon/compare/{bpan_b}", CompareCarbonFootprints(carbonService))
}
