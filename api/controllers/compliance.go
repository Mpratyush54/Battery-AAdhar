// compliance.go — Compliance verification and audit

package controllers

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
	"github.com/go-chi/chi/v5"
)

// CheckCompliance godoc
// @Summary      Check battery compliance status
// @Description  Returns the current compliance status against BPA regulations
// @Tags         compliance
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {object}  map[string]interface{}
// @Failure      404  {object}  map[string]string  "Battery not found"
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/compliance [get]
// @Security     BearerAuth
func CheckCompliance(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetViolations godoc
// @Summary      Get compliance violations
// @Description  Returns all recorded compliance violations for a battery
// @Tags         compliance
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {array}   map[string]interface{}
// @Failure      404  {object}  map[string]string
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/violations [get]
// @Security     BearerAuth
func GetViolations(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetAuditTrail godoc
// @Summary      Get audit trail
// @Description  Returns the hash-chain audit trail for a battery (government/admin only)
// @Tags         compliance
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {array}   map[string]interface{}
// @Failure      403  {object}  map[string]string  "Forbidden"
// @Failure      404  {object}  map[string]string
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/audit [get]
// @Security     BearerAuth
func GetAuditTrail(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetBatteryCompliance — GET /api/v1/batteries/{bpan}/compliance
// @Summary Check battery compliance status
// @Description Returns violations if any, categorized by severity
// @Tags compliance
// @Param bpan path string true "BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} models.ComplianceStatusResponse
// @Router /api/v1/batteries/{bpan}/compliance [get]
func GetBatteryCompliance(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")

		status, err := complianceService.GetComplianceStatus(r.Context(), bpan)
		if err != nil {
			slog.Error("compliance check failed", "bpan", bpan, "error", err)
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "battery not found"})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(status)
	}
}

// TriggerComplianceScan — POST /api/v1/compliance/scan
// @Summary Scan all batteries for compliance violations
// @Description Requires REGULATOR or ADMIN role (background job)
// @Tags compliance
// @Accept json
// @Produce json
// @Success 202 {object} map[string]string "Scan initiated"
// @Failure 403 {object} map[string]string "Not regulator"
// @Router /api/v1/compliance/scan [post]
// @Security Bearer
func TriggerComplianceScan(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		claims := middleware.ClaimsFromContext(r.Context())

		// Only regulator can trigger full scan
		if claims.Role != "regulator" && claims.Role != "admin" {
			w.WriteHeader(http.StatusForbidden)
			json.NewEncoder(w).Encode(map[string]string{
				"error": "only regulator can trigger compliance scan",
			})
			return
		}

		// Start scan in background
		scanID, err := complianceService.StartComplianceScan(r.Context())
		if err != nil {
			slog.Error("scan start failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			return
		}

		slog.Info("compliance scan triggered",
			"actor", claims.Subject,
			"scan_id", scanID,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusAccepted) // 202 Accepted
		json.NewEncoder(w).Encode(map[string]interface{}{
			"scan_id": scanID,
			"status":  "in_progress",
			"message": "Compliance scan initiated. Results will be available shortly.",
		})
	}
}

// VerifyOperational — POST /api/v1/compliance/verify/operational
// @Summary Verify battery is operational via ZK proof (no value disclosure)
// @Description Government regulator can verify SoH > 80% without seeing actual SoH
// @Tags compliance
// @Param bpan query string true "BPAN to verify"
// @Accept json
// @Produce json
// @Success 200 {object} models.ComplianceProofResponse "Proof + commitment"
// @Failure 403 {object} map[string]string "Not regulator/government"
// @Router /api/v1/compliance/verify/operational [post]
// @Security Bearer
func VerifyOperational(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := r.URL.Query().Get("bpan")
		claims := middleware.ClaimsFromContext(r.Context())

		// Only government/regulator can verify
		if claims.Role != "regulator" && claims.Role != "government" && claims.Role != "admin" {
			w.WriteHeader(http.StatusForbidden)
			json.NewEncoder(w).Encode(map[string]string{
				"error": "only government regulator can verify compliance",
			})
			return
		}

		if bpan == "" {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "bpan required"})
			return
		}

		// Generate ZK proof
		proof, commitment, err := complianceService.GenerateComplianceProof(
			r.Context(),
			bpan,
			"operational",
		)
		if err != nil {
			slog.Error("proof generation failed", "bpan", bpan, "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "proof generation failed"})
			return
		}

		slog.Info("compliance proof generated",
			"bpan", bpan,
			"requirement", "operational",
			"verifier", claims.Subject,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(models.ComplianceProofResponse{
			BPAN:        bpan,
			Requirement: "operational",
			Statement:   "Battery SoH > 80% (battery is OPERATIONAL)",
			Proof:       proof,
			Commitment:  commitment,
			Note:        "This proof was generated without revealing the actual SoH value to the verifier",
		})
	}
}

// VerifySecondLife — POST /api/v1/compliance/verify/second-life
// @Summary Verify battery eligible for second-life (SoH 60–80%)
// @Tags compliance
// @Param bpan query string true "BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} models.ComplianceProofResponse
// @Router /api/v1/compliance/verify/second-life [post]
// @Security Bearer
func VerifySecondLife(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := r.URL.Query().Get("bpan")
		claims := middleware.ClaimsFromContext(r.Context())

		if claims.Role != "regulator" && claims.Role != "admin" {
			w.WriteHeader(http.StatusForbidden)
			return
		}

		proof, commitment, err := complianceService.GenerateComplianceProof(
			r.Context(),
			bpan,
			"second_life",
		)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(models.ComplianceProofResponse{
			BPAN:        bpan,
			Requirement: "second_life",
			Statement:   "Battery SoH >= 60% (eligible for SECOND_LIFE)",
			Proof:       proof,
			Commitment:  commitment,
		})
	}
}

// GetComplianceDashboard — GET /api/v1/dashboard/compliance
// @Summary Government compliance dashboard (aggregated violations)
// @Tags dashboard
// @Accept json
// @Produce json
// @Success 200 {object} models.ComplianceDashboard
// @Router /api/v1/dashboard/compliance [get]
// @Security Bearer
func GetComplianceDashboard(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		claims := middleware.ClaimsFromContext(r.Context())

		if claims.Role != "regulator" && claims.Role != "admin" {
			w.WriteHeader(http.StatusForbidden)
			return
		}

		dashboard, err := complianceService.GetComplianceDashboard(r.Context())
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(dashboard)
	}
}

// RegisterComplianceRoutes registers all compliance endpoints
func RegisterComplianceRoutes(r chi.Router, complianceService *services.ComplianceService) {
	r.Get("/batteries/{bpan}/compliance", GetBatteryCompliance(complianceService))
	r.Post("/compliance/scan", TriggerComplianceScan(complianceService))
	r.Post("/compliance/verify/operational", VerifyOperational(complianceService))
	r.Post("/compliance/verify/second-life", VerifySecondLife(complianceService))
	r.Get("/dashboard/compliance", GetComplianceDashboard(complianceService))
}
