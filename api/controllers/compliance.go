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
func GetBatteryCompliance(complianceService *services.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")

		// For now, return a basic compliance check
		// TODO: Wire to full Rust compliance service
		status, err := complianceService.CheckCompliance(r.Context(), bpan, 100.0, true, true)
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

// VerifyComplianceOperational — POST /api/v1/compliance/verify/operational
// Generates a ZK proof that battery meets operational standards.
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

// GetComplianceDashboard — GET /api/v1/dashboard/compliance
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
	r.Get("/dashboard/compliance", GetComplianceDashboard(complianceService))
}
