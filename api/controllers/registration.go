// registration.go — Battery registration endpoint (atomic, linking all data)

package controllers

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
)

// RegisterBattery — POST /api/v1/batteries/register
// @Summary Register new battery with all static data
// @Description Atomic registration: descriptor + BMCS + BCF + initial health
// @Tags battery
// @Param body body models.BatteryRegistrationRequest true "Complete battery data"
// @Accept json
// @Produce json
// @Success 201 {object} map[string]string "BPAN generated"
// @Failure 400 {object} map[string]string "Invalid data"
// @Router /api/v1/batteries/register [post]
// @Security Bearer
func RegisterBattery(registrationService *services.RegistrationService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		claims := middleware.ClaimsFromContext(r.Context())

		// Only manufacturer can register
		if claims.Role != "manufacturer" && claims.Role != "admin" {
			w.WriteHeader(http.StatusForbidden)
			json.NewEncoder(w).Encode(map[string]string{
				"error": "only manufacturer can register batteries",
			})
			return
		}

		var req models.BatteryRegistrationRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		// Call Rust service for atomic registration
		bpan, err := registrationService.RegisterBattery(
			r.Context(),
			&req,
			claims.Subject,
		)
		if err != nil {
			slog.Error("registration failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "registration failed"})
			return
		}

		slog.Info("battery registered",
			"bpan", bpan,
			"manufacturer", claims.Subject,
			"capacity_kwh", req.Descriptor.CapacityKwh,
		)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]interface{}{
			"bpan":                bpan,
			"status":              "REGISTERED",
			"lifecycle_state":     "REGISTERED",
			"battery_hash":        "computed",
		})
	}
}
