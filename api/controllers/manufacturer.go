package controllers

import (
	"encoding/json"
	"log/slog"
	"net/http"
	"strconv"

	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
	"github.com/go-chi/chi/v5"
)

// RegisterManufacturerRoutes wires all manufacturer endpoints onto the router.
func RegisterManufacturerRoutes(r chi.Router, s *services.ManufacturerService) {
	r.Group(func(r chi.Router) {
		r.Use(middleware.Authenticate)

		// Manufacturer self-service (MANUFACTURER role)
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireResource(models.ResourceManufacturer, models.ActionRead))

			// GET /manufacturer/batteries — list own batteries
			r.Get("/manufacturer/batteries", ListOwnBatteries(s))

			// GET /manufacturer/dashboard — dashboard aggregates
			r.Get("/manufacturer/dashboard", ManufacturerDashboard(s))

			// POST /manufacturer/batteries/batch — batch register batteries
			r.Post("/manufacturer/batteries/batch", BatchRegisterBatteries(s))

			// POST /manufacturer/battery/{bpan}/material — submit BMCS
			r.Post("/manufacturer/battery/{bpan}/material", SubmitMaterial(s))

			// POST /manufacturer/battery/{bpan}/carbon — submit BCF
			r.Post("/manufacturer/battery/{bpan}/carbon", SubmitCarbon(s))
		})

		// Admin-only endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.IsRole("admin"))

			// POST /manufacturer/register — register new manufacturer
			r.Post("/manufacturer/register", RegisterManufacturer(s))

			// GET /manufacturers — list all manufacturers
			r.Get("/manufacturers", ListManufacturers(s))
		})
	})
}

// RegisterManufacturer — POST /manufacturer/register
// @Summary Register a new manufacturer
// @Description Regulator (admin) registers a new manufacturer with profile data encrypted by manufacturer-specific DEK
// @Tags manufacturer
// @Accept json
// @Produce json
// @Param body body models.RegisterManufacturerRequest true "Manufacturer registration payload"
// @Success 201 {object} models.RegisterManufacturerResponse
// @Failure 400 {object} map[string]string
// @Failure 409 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Router /manufacturer/register [post]
// @Security Bearer
func RegisterManufacturer(s *services.ManufacturerService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "manufacturer service unavailable"})
			return
		}

		var req models.RegisterManufacturerRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		regulatorID := middleware.GetUserID(r)

		resp, err := s.RegisterManufacturer(r.Context(), &req, regulatorID)
		if err != nil {
			slog.Error("manufacturer registration failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(resp)
	}
}

// ListManufacturers — GET /manufacturers
// @Summary List all manufacturers
// @Description Admin endpoint to list all registered manufacturers
// @Tags manufacturer
// @Produce json
// @Success 200 {array} models.ManufacturerProfile
// @Failure 500 {object} map[string]string
// @Router /manufacturers [get]
// @Security Bearer
func ListManufacturers(s *services.ManufacturerService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "manufacturer service unavailable"})
			return
		}

		list, err := s.ListManufacturers(r.Context())
		if err != nil {
			slog.Error("list manufacturers failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		json.NewEncoder(w).Encode(list)
	}
}

// BatchRegisterBatteries — POST /manufacturer/batteries/batch
// @Summary Batch register batteries from CSV
// @Description Manufacturer submits CSV data to register multiple batteries in a single transaction
// @Tags manufacturer
// @Accept json
// @Produce json
// @Param body body models.BatchBatteryRequest true "Batch battery data"
// @Success 201 {object} models.BatchBatteryResponse
// @Failure 400 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Router /manufacturer/batteries/batch [post]
// @Security Bearer
func BatchRegisterBatteries(s *services.ManufacturerService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "manufacturer service unavailable"})
			return
		}

		var req models.BatchBatteryRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		claims := middleware.ClaimsFromContext(r.Context())
		actorID := claims.Subject

		resp, err := s.BatchRegisterBatteries(r.Context(), &req, actorID)
		if err != nil {
			slog.Error("batch registration failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(resp)
	}
}

// ListOwnBatteries — GET /manufacturer/batteries
// @Summary List manufacturer's own batteries
// @Description Returns paginated list of batteries registered by this manufacturer
// @Tags manufacturer
// @Produce json
// @Param limit query int false "Page size" default(50)
// @Param offset query int false "Page offset" default(0)
// @Success 200 {array} models.ManufacturerBatterySummary
// @Failure 500 {object} map[string]string
// @Router /manufacturer/batteries [get]
// @Security Bearer
func ListOwnBatteries(s *services.ManufacturerService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "manufacturer service unavailable"})
			return
		}

		limitStr := r.URL.Query().Get("limit")
		offsetStr := r.URL.Query().Get("offset")
		limit := int64(50)
		offset := int64(0)
		if l, err := strconv.ParseInt(limitStr, 10, 64); err == nil && l > 0 && l <= 500 {
			limit = l
		}
		if o, err := strconv.ParseInt(offsetStr, 10, 64); err == nil && o >= 0 {
			offset = o
		}

		claims := middleware.ClaimsFromContext(r.Context())

		batteries, err := s.ListOwnBatteries(r.Context(), claims.Subject, limit, offset)
		if err != nil {
			slog.Error("list batteries failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		json.NewEncoder(w).Encode(batteries)
	}
}

// ManufacturerDashboard — GET /manufacturer/dashboard
// @Summary Get manufacturer dashboard
// @Description Returns aggregated dashboard data for the authenticated manufacturer
// @Tags manufacturer
// @Produce json
// @Success 200 {object} models.ManufacturerDashboard
// @Failure 500 {object} map[string]string
// @Router /manufacturer/dashboard [get]
// @Security Bearer
func ManufacturerDashboard(s *services.ManufacturerService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "manufacturer service unavailable"})
			return
		}

		claims := middleware.ClaimsFromContext(r.Context())

		dashboard, err := s.GetDashboard(r.Context(), claims.Subject)
		if err != nil {
			slog.Error("dashboard failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		json.NewEncoder(w).Encode(dashboard)
	}
}

// SubmitMaterial — POST /manufacturer/battery/{bpan}/material
// @Summary Submit battery material composition data
// @Description Manufacturer submits BMCS data for a specific battery
// @Tags manufacturer
// @Accept json
// @Produce json
// @Param bpan path string true "BPAN"
// @Success 201 {object} map[string]interface{}
// @Failure 400 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Router /manufacturer/battery/{bpan}/material [post]
// @Security Bearer
func SubmitMaterial(s *services.ManufacturerService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "manufacturer service unavailable"})
			return
		}

		bpan := chi.URLParam(r, "bpan")

		var matReq models.MaterialCompositionRequest
		if err := json.NewDecoder(r.Body).Decode(&matReq); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		err := s.SubmitMaterial(r.Context(), bpan, &matReq)
		if err != nil {
			slog.Error("material submission failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]interface{}{
			"bpan":   bpan,
			"status": "material_submitted",
		})
	}
}

// SubmitCarbon — POST /manufacturer/battery/{bpan}/carbon
// @Summary Submit battery carbon footprint data
// @Description Manufacturer submits BCF data for a specific battery
// @Tags manufacturer
// @Accept json
// @Produce json
// @Param bpan path string true "BPAN"
// @Success 201 {object} map[string]interface{}
// @Failure 400 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Router /manufacturer/battery/{bpan}/carbon [post]
// @Security Bearer
func SubmitCarbon(s *services.ManufacturerService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "manufacturer service unavailable"})
			return
		}

		bpan := chi.URLParam(r, "bpan")

		var req models.CarbonFootprintRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		err := s.SubmitCarbon(r.Context(), bpan, &req)
		if err != nil {
			slog.Error("carbon submission failed", "error", err)
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]interface{}{
			"bpan":   bpan,
			"status": "carbon_submitted",
		})
	}
}
