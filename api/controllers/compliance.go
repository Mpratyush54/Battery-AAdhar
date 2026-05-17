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

// GetBatteryCompliance — GET /api/v1/batteries/{bpan}/compliance
// @Summary Get battery compliance status
// @Description Returns compliance status and violations for a battery
// @Tags compliance
// @Param bpan path string true "BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} models.ComplianceStatusResponse
// @Failure 500 {object} map[string]string "Internal server error"
// @Router /api/v1/batteries/{bpan}/compliance [get]
// @Security Bearer
func GetBatteryCompliance(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")

		status, err := complianceService.GetComplianceStatus(r.Context(), bpan)
		if err != nil {
			slog.Error("compliance check failed", "bpan", bpan, "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(status)
	}
}

// TriggerComplianceScan — POST /api/v1/compliance/scan
// @Summary Trigger compliance scan for all batteries
// @Description Scans all batteries for compliance violations (regulator only)
// @Tags compliance
// @Accept json
// @Produce json
// @Success 202 {object} map[string]interface{} "Scan completed"
// @Failure 403 {object} map[string]string "Forbidden (non-regulator)"
// @Router /api/v1/compliance/scan [post]
// @Security Bearer
func TriggerComplianceScan(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		claims := middleware.ClaimsFromContext(r.Context())

		if claims.Role != "regulator" && claims.Role != "admin" {
			w.WriteHeader(http.StatusForbidden)
			json.NewEncoder(w).Encode(map[string]string{
				"error": "only regulator can trigger compliance scan",
			})
			return
		}

		result, err := complianceService.TriggerComplianceScan(r.Context())
		if err != nil {
			slog.Error("scan start failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			return
		}

		slog.Info("compliance scan triggered",
			"actor", claims.Subject,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusAccepted)
		json.NewEncoder(w).Encode(map[string]interface{}{
			"total_batteries": result.TotalBatteries,
			"status":          "completed",
		})
	}
}

// VerifyComplianceOperational — POST /api/v1/batteries/{bpan}/verify/operational
// @Summary Verify battery operational compliance (ZK proof)
// @Description Generates a ZK proof that battery meets operational standards (government/regulator only)
// @Tags compliance
// @Param bpan path string true "BPAN"
// @Produce json
// @Success 200 {object} models.ComplianceProofResponse
// @Failure 400 {object} map[string]string "Bad request"
// @Failure 403 {object} map[string]string "Forbidden"
// @Router /api/v1/batteries/{bpan}/verify/operational [post]
// @Security Bearer
func VerifyComplianceOperational(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")
		claims := middleware.ClaimsFromContext(r.Context())

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

		// Generate ZK proof via lifecycle service
		proof, err := complianceService.VerifyOperational(r.Context(), bpan, 85.0)
		if err != nil {
			slog.Error("proof generation failed", "bpan", bpan, "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "proof generation failed"})
			return
		}

		slog.Info("compliance proof generated",
			"bpan", bpan,
			"verifier", claims.Subject,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(models.ComplianceProofResponse{
			BPAN:        bpan,
			Requirement: "operational",
			Statement:   "Battery SoH > 80% (battery is OPERATIONAL)",
			Proof:       proof.Proof,
			Commitment:  proof.Commitment,
			Note:        "This proof was generated without revealing the actual SoH value",
		})
	}
}

// VerifyComplianceSecondLife — POST /api/v1/batteries/{bpan}/verify/second-life
// @Summary Verify second-life eligibility (ZK proof)
// @Description Generates a ZK proof that battery is eligible for second-life use (government/regulator only)
// @Tags compliance
// @Param bpan path string true "BPAN"
// @Produce json
// @Success 200 {object} models.ComplianceProofResponse
// @Failure 400 {object} map[string]string "Bad request"
// @Failure 403 {object} map[string]string "Forbidden"
// @Router /api/v1/batteries/{bpan}/verify/second-life [post]
// @Security Bearer
func VerifyComplianceSecondLife(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")
		claims := middleware.ClaimsFromContext(r.Context())

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

		proof, err := complianceService.VerifySecondLife(r.Context(), bpan)
		if err != nil {
			slog.Error("second-life proof generation failed", "bpan", bpan, "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "proof generation failed"})
			return
		}

		slog.Info("second-life compliance proof generated",
			"bpan", bpan,
			"verifier", claims.Subject,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(models.ComplianceProofResponse{
			BPAN:        bpan,
			Requirement: "second_life",
			Statement:   "Battery SoH >= 60% (eligible for SECOND_LIFE)",
			Proof:       proof.Proof,
			Commitment:  proof.Commitment,
			Note:        "This proof was generated without revealing the actual SoH value",
		})
	}
}

// GetComplianceDashboard — GET /api/v1/dashboard/compliance
// @Summary Get compliance dashboard
// @Description Returns aggregated compliance statistics (regulator/admin only)
// @Tags compliance
// @Produce json
// @Success 200 {object} models.ComplianceDashboard
// @Failure 403 {object} map[string]string "Forbidden"
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
	r.Post("/batteries/{bpan}/verify/operational", VerifyComplianceOperational(complianceService))
	r.Post("/batteries/{bpan}/verify/second-life", VerifyComplianceSecondLife(complianceService))
	r.Get("/dashboard/compliance", GetComplianceDashboard(complianceService))
}
