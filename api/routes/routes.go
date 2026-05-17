// routes.go — chi-based router for the Battery Aadhaar API
// Replaces the previous http.ServeMux implementation.
// All existing route paths are preserved; only the router type changes.
package routes

import (
	"net/http"

	"github.com/go-chi/chi/v5"
	chiMiddleware "github.com/go-chi/chi/v5/middleware"
	httpSwagger "github.com/swaggo/http-swagger"

	"github.com/Mpratyush54/Battery-AAdhar/api/config"
	"github.com/Mpratyush54/Battery-AAdhar/api/controllers"
	_ "github.com/Mpratyush54/Battery-AAdhar/api/docs"
	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
)

// NewRouter constructs and returns the application chi.Router.
// All middleware is applied here in the correct order:
//  1. chi built-ins (request ID, real IP, recoverer)
//  2. custom logging   (structured slog output)
//  3. custom auth      (JWT parse + attach claims to context)
//  4. custom RBAC      (role enforcement per route group)
func NewRouter(microservices *config.MicroserviceClients) http.Handler {

	r := chi.NewRouter()

	// ── Global middleware (runs on every request) ─────────────────────────
	r.Use(chiMiddleware.RequestID)
	r.Use(chiMiddleware.RealIP)
	r.Use(chiMiddleware.Recoverer)
	r.Use(middleware.Logger)       // structured logging
	r.Use(middleware.Authenticate) // JWT parse — attaches claims to context

	// ── Health / readiness ────────────────────────────────────────────────
	r.Get("/healthz", handleHealthz)
	r.Get("/readyz", handleReadyz)

	// ── Swagger UI ────────────────────────────────────────────────────────
	r.Get("/swagger/*", httpSwagger.Handler())

	// ── API v1 ────────────────────────────────────────────────────────────
	r.Route("/api/v1", func(r chi.Router) {

		// Auth endpoints (no RBAC — public)
		r.Route("/auth", func(r chi.Router) {
			r.Post("/register", controllers.RegisterStakeholderController)
			r.Post("/login", controllers.LoginController)
			r.Post("/refresh", controllers.RefreshController)
			r.Post("/logout", controllers.LogoutController)
		})

		// Public endpoints — no auth required beyond claim parse
		r.Group(func(r chi.Router) {
			r.Get("/battery", GetBatteryByQuery)
			r.Get("/batteries/{bpan}", controllers.GetBatteryByBPAN)
		})

		// Battery registration (manufacturer only — RBAC enforced by middleware)
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireResource(models.ResourceBattery, models.ActionCreate))
			if microservices != nil {
				regSvc := services.NewRegistrationService(microservices.GrpcConn.RawConn())
				r.Post("/battery/register", controllers.RegisterBattery(regSvc))
			} else {
				r.Post("/battery/register", handleRegistrationUnavailable)
			}
		})

		// Service provider endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireResource(models.ResourceBatteryHealth, models.ActionUpdate))
			r.Patch("/batteries/{bpan}/status", UpdateBatteryStatus)
		})

		// Compliance / ZK verification endpoints (verifier role)
		r.Group(func(r chi.Router) {
			r.Use(middleware.IsRole("verifier"))
			if microservices != nil {
			r.Post("/batteries/{bpan}/verify/operational", controllers.VerifyOperationalHandler(microservices.GrpcConn))
			r.Post("/batteries/{bpan}/verify/signature", controllers.VerifySignature(microservices.GrpcConn))
			} else {
				r.Post("/batteries/{bpan}/verify/operational", handleServiceUnavailable)
				r.Post("/batteries/{bpan}/verify/signature", handleServiceUnavailable)
			}
		})

		// Admin-only
		r.Group(func(r chi.Router) {
			r.Use(middleware.IsRole("admin"))
			r.Post("/manufacturers", RegisterManufacturer)
			r.Get("/manufacturers", ListManufacturers)
		})

		// ── Controller-based routes (each controller registers its own group) ──
		var healthSvc *services.HealthService
		var carbonSvc *services.CarbonService
		var qrSvc *services.QrService
		var batterySvc *services.BatteryService
		var complianceSvc *services.ComplianceService
		var encSvc *services.EncryptionService

		if microservices != nil {
			healthSvc = services.NewHealthServiceWithClient(microservices.GrpcConn)
			carbonSvc = services.NewCarbonServiceWithClient(microservices.GrpcConn)
			qrSvc = services.NewQrServiceWithClient(microservices.GrpcConn)
			encSvc = services.NewEncryptionService(microservices.GrpcConn.CryptoClient)
			batterySvc = services.NewBatteryServiceWithClient(microservices.GrpcConn, encSvc)
			complianceSvc = services.NewComplianceServiceWithClient(microservices.GrpcConn)
		} else {
			healthSvc = services.NewHealthService()
			carbonSvc = services.NewCarbonService()
			qrSvc = services.NewQrService()
			batterySvc = services.NewBatteryService(nil)
			complianceSvc = services.NewComplianceService()
		}

		controllers.RegisterMaterialRoutes(r)
		controllers.RegisterCarbonRoutes(r, carbonSvc)
		controllers.RegisterHealthRoutes(r, healthSvc)

		// Lifecycle routes (ownership transfers, state transitions)
		if microservices != nil {
			controllers.RegisterLifecycleRoutes(r, services.NewLifecycleService(microservices.GrpcConn))
		} else {
			controllers.RegisterLifecycleRoutes(r, nil)
		}

		// Compliance routes (scan, violations, ZK proofs)
		controllers.RegisterComplianceRoutes(r, complianceSvc)

		// Telemetry routes (ingest, query, history)
		controllers.RegisterTelemetryRoutes(r)

		// QR code routes (generate, validate, public lookup)
		controllers.RegisterQRRoutes(r, qrSvc)

		// Public endpoints — no authentication required
		controllers.RegisterPublicRoutes(r, batterySvc, qrSvc)

		// Circular Economy (Reuse/Recycling)
		if microservices != nil {
			controllers.RegisterReuseRecyclingRoutes(r,
				services.NewReuseService(microservices.GrpcConn),
				services.NewRecyclingService(microservices.GrpcConn),
				services.NewDashboardService(microservices.GrpcConn),
			)
		} else {
			controllers.RegisterReuseRecyclingRoutes(r, nil, nil, nil)
		}
	})

	return r
}

// ── Fallback handlers when services are unavailable ──

func handleHealthz(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte(`{"status":"ok"}`))
}

func handleReadyz(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte(`{"status":"ready"}`))
}

func handleUpdateStatus(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}

func handleRegisterManufacturer(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}

func handleListManufacturers(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}

func handleRegistrationUnavailable(w http.ResponseWriter, r *http.Request) {
	http.Error(w, `{"error":"registration service unavailable — Rust gRPC engine not connected"}`, http.StatusServiceUnavailable)
}

func handleServiceUnavailable(w http.ResponseWriter, r *http.Request) {
	http.Error(w, `{"error":"service unavailable — Rust gRPC engine not connected"}`, http.StatusServiceUnavailable)
}

// GetBatteryByQuery — GET /api/v1/battery
// @Summary Get battery by query parameter
// @Description Retrieve battery information by BPAN query parameter
// @Tags battery
// @Param bpan query string true "BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} map[string]interface{}
// @Failure 400 {object} map[string]string
// @Router /api/v1/battery [get]
func GetBatteryByQuery(w http.ResponseWriter, r *http.Request) {
	controllers.GetBatteryController(w, r)
}

// UpdateBatteryStatus — PATCH /api/v1/batteries/{bpan}/status
// @Summary Update battery status
// @Description Update battery lifecycle status (service provider only)
// @Tags battery
// @Param bpan path string true "BPAN"
// @Accept json
// @Produce json
// @Success 200 {object} map[string]interface{}
// @Router /api/v1/batteries/{bpan}/status [patch]
// @Security Bearer
func UpdateBatteryStatus(w http.ResponseWriter, r *http.Request) {
	handleUpdateStatus(w, r)
}

// RegisterManufacturer — POST /api/v1/manufacturers
// @Summary Register manufacturer
// @Description Register a new manufacturer (admin only)
// @Tags manufacturer
// @Accept json
// @Produce json
// @Success 201 {object} map[string]interface{}
// @Router /api/v1/manufacturers [post]
// @Security Bearer
func RegisterManufacturer(w http.ResponseWriter, r *http.Request) {
	handleRegisterManufacturer(w, r)
}

// ListManufacturers — GET /api/v1/manufacturers
// @Summary List manufacturers
// @Description List all registered manufacturers (admin only)
// @Tags manufacturer
// @Produce json
// @Success 200 {array} map[string]interface{}
// @Router /api/v1/manufacturers [get]
// @Security Bearer
func ListManufacturers(w http.ResponseWriter, r *http.Request) {
	handleListManufacturers(w, r)
}
