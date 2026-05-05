// health.go — Battery health endpoints

package controllers

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"

	"github.com/Mpratyush54/Battery-AAdhar/api/config"
	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
	"github.com/go-chi/chi/v5"
)

// UpdateHealth — PATCH /api/v1/batteries/{bpan}/health
// @Summary Update battery health (SoH, cycles, degradation)
// @Description Submit health update from BMS or manufacturer
// @Tags health
// @Param bpan path string true "BPAN"
// @Param body body models.HealthUpdateRequest true "Health data"
// @Accept json
// @Produce json
// @Success 201 {object} map[string]string "Record ID"
// @Failure 429 {object} map[string]string "Rate limited"
// @Router /api/v1/batteries/{bpan}/health [patch]
// @Security Bearer
func UpdateHealth(healthService *services.HealthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")
		claims := middleware.ClaimsFromContext(r.Context())

		var req models.HealthUpdateRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		recordID, err := healthService.UpdateHealth(r.Context(), bpan, &req, claims.Role)
		if err != nil {
			slog.Error("update health failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "update failed"})
			return
		}

		slog.Info("health updated",
			"bpan", bpan,
			"soh", req.StateOfHealthPercent,
			"cycles", req.CycleCount,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]interface{}{
			"record_id": recordID,
			"bpan":      bpan,
			"soh":       req.StateOfHealthPercent,
		})
	}
}

// GetCurrentHealth — GET /api/v1/batteries/{bpan}/health
// @Summary Get current battery health status
// @Tags health
// @Param bpan path string true "BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} models.HealthRecord
// @Router /api/v1/batteries/{bpan}/health [get]
func GetCurrentHealth(healthService *services.HealthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")

		record, err := healthService.GetCurrentHealth(r.Context(), bpan)
		if err != nil {
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "not found"})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(record)
	}
}

// GetHealthHistory — GET /api/v1/batteries/{bpan}/health/history
// @Summary Get battery health time-series
// @Tags health
// @Param bpan path string true "BPAN"
// @Param limit query int false "Max records (default 100)"
// @Accept json
// @Produce json
// @Success 200 {array} models.HealthRecord
// @Router /api/v1/batteries/{bpan}/health/history [get]
func GetHealthHistory(healthService *services.HealthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bpan := chi.URLParam(r, "bpan")
		limit := 100 // default

		if l := r.URL.Query().Get("limit"); l != "" {
			fmt.Sscanf(l, "%d", &limit)
		}

		history, err := healthService.GetHealthHistory(r.Context(), bpan, int32(limit))
		if err != nil {
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "not found"})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(history)
	}
}

// GetHealthDashboard — GET /api/v1/health/dashboard
// @Summary Get battery health dashboard metrics
// @Tags health
// @Accept json
// @Produce json
// @Success 200 {object} models.HealthDashboard
// @Router /api/v1/health/dashboard [get]
func GetHealthDashboard(healthService *services.HealthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		dashboard, err := healthService.GetDashboard(r.Context())
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "dashboard failed"})
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(dashboard)
	}
}

// RegisterHealthRoutes registers health endpoints
func RegisterHealthRoutes(r chi.Router, healthService *services.HealthService) {
	rateLimiter := middleware.NewRateLimiter(config.RedisClient)
	rateLimitMiddleware := middleware.HealthUpdateRateLimitMiddleware(rateLimiter)

	r.With(rateLimitMiddleware).Patch("/batteries/{bpan}/health", UpdateHealth(healthService))
	r.Get("/batteries/{bpan}/health", GetCurrentHealth(healthService))
	r.Get("/batteries/{bpan}/health/history", GetHealthHistory(healthService))
	r.Get("/health/dashboard", GetHealthDashboard(healthService))
}
