package controllers

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
)

// RegisterReuseRecyclingRoutes wires up the endpoints for reuse certification, recycling records, and the circular economy dashboard.
func RegisterReuseRecyclingRoutes(r chi.Router, reuseSvc *services.ReuseService, recycleSvc *services.RecyclingService, dashboardSvc *services.DashboardService) {
	r.Route("/circular-economy", func(r chi.Router) {
		// Reuse routes
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireResource(models.ResourceBatteryLifecycle, models.ActionUpdate))
			r.Post("/batteries/{bpan}/reuse", handleCertifyReuse(reuseSvc))
		})

		// Recycling routes
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireResource(models.ResourceBatteryLifecycle, models.ActionUpdate))
			r.Post("/batteries/{bpan}/recycle", handleRecordRecycling(recycleSvc))
		})

		// Dashboard routes (Public or Authenticated)
		r.Get("/dashboard", handleGetCircularEconomyMetrics(dashboardSvc))
	})
}

// handleCertifyReuse handles POST /api/v1/circular-economy/batteries/{bpan}/reuse
// @Summary      Certify battery for second-life reuse
// @Description  Transitions state to SECOND_LIFE (reuse operator only)
// @Tags         circular-economy
// @Accept       json
// @Produce      json
// @Param        bpan  path  string  true  "Battery PAN"
// @Param        body  body  models.ReuseCertificationRequest  true  "Reuse certification payload"
// @Success      201  {object}  map[string]interface{}
// @Failure      400  {object}  string
// @Failure      503  {object}  string
// @Router       /api/v1/circular-economy/batteries/{bpan}/reuse [post]
// @Security     BearerAuth
func handleCertifyReuse(svc *services.ReuseService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if svc == nil {
			http.Error(w, "Circular Economy service unavailable (gRPC core disconnected)", http.StatusServiceUnavailable)
			return
		}

		bpan := chi.URLParam(r, "bpan")
		if bpan == "" {
			http.Error(w, "bpan is required", http.StatusBadRequest)
			return
		}

		var req models.ReuseCertificationRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "Invalid request payload", http.StatusBadRequest)
			return
		}

		claims := middleware.ClaimsFromContext(r.Context())
		certifiedBy := "system"
		if claims != nil {
			certifiedBy = claims.Subject
		}

		certID, err := svc.CertifySecondLife(r.Context(), bpan, req.SohPercent, certifiedBy, req.Application, req.ExpectedYears)
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]string{
			"status":           "success",
			"certification_id": certID,
			"message":          "Second-life certification issued",
		})
	}
}

// handleRecordRecycling handles POST /api/v1/circular-economy/batteries/{bpan}/recycle
// @Summary      Record battery recycling
// @Description  Transitions state to RECYCLED (recycler only)
// @Tags         circular-economy
// @Accept       json
// @Produce      json
// @Param        bpan  path  string  true  "Battery PAN"
// @Param        body  body  models.RecyclingRecordRequest  true  "Recycling record payload"
// @Success      201  {object}  map[string]interface{}
// @Failure      400  {object}  string
// @Failure      503  {object}  string
// @Router       /api/v1/circular-economy/batteries/{bpan}/recycle [post]
// @Security     BearerAuth
func handleRecordRecycling(svc *services.RecyclingService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if svc == nil {
			http.Error(w, "Circular Economy service unavailable (gRPC core disconnected)", http.StatusServiceUnavailable)
			return
		}

		bpan := chi.URLParam(r, "bpan")
		if bpan == "" {
			http.Error(w, "bpan is required", http.StatusBadRequest)
			return
		}

		var req models.RecyclingRecordRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "Invalid request payload", http.StatusBadRequest)
			return
		}

		claims := middleware.ClaimsFromContext(r.Context())
		recycledBy := "system"
		if claims != nil {
			recycledBy = claims.Subject
		}

		certID, err := svc.RecordRecycling(r.Context(), bpan, recycledBy, req.Method, req.WeightKg, req.Standard, req.RecoveryRates)
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]string{
			"status":           "success",
			"certification_id": certID,
			"message":          "Recycling record submitted",
		})
	}
}

// handleGetCircularEconomyMetrics handles GET /api/v1/circular-economy/dashboard
// @Summary      Get circular economy metrics dashboard
// @Description  Returns aggregated reuse and recycling metrics
// @Tags         circular-economy
// @Produce      json
// @Param        manufacturer_id  query  string  false  "Filter by manufacturer"
// @Param        chemistry_type   query  string  false  "Filter by chemistry"
// @Success      200  {object}  map[string]interface{}
// @Failure      503  {object}  string
// @Router       /api/v1/circular-economy/dashboard [get]
func handleGetCircularEconomyMetrics(svc *services.DashboardService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if svc == nil {
			http.Error(w, "Circular Economy service unavailable (gRPC core disconnected)", http.StatusServiceUnavailable)
			return
		}

		manufacturerID := r.URL.Query().Get("manufacturer_id")
		chemistryType := r.URL.Query().Get("chemistry_type")

		metrics, err := svc.GetCircularEconomyMetrics(r.Context(), manufacturerID, chemistryType)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(metrics)
	}
}
