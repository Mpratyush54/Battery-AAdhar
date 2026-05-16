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
			r.Get("/battery", controllers.GetBatteryController)
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
			r.Patch("/batteries/{bpan}/status", handleUpdateStatus)
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
			r.Post("/manufacturers", handleRegisterManufacturer)
			r.Get("/manufacturers", handleListManufacturers)
		})

		// ── Controller-based routes (each controller registers its own group) ──
		controllers.RegisterMaterialRoutes(r)
		controllers.RegisterCarbonRoutes(r)
		controllers.RegisterHealthRoutes(r, services.NewHealthService())

		// Lifecycle routes (ownership transfers, state transitions)
		if microservices != nil {
			controllers.RegisterLifecycleRoutes(r, services.NewLifecycleService(microservices.GrpcConn))
		} else {
			controllers.RegisterLifecycleRoutes(r, nil)
		}

		// Compliance routes (scan, violations, ZK proofs)
		controllers.RegisterComplianceRoutes(r, services.NewComplianceService())

		// Telemetry routes (ingest, query, history)
		controllers.RegisterTelemetryRoutes(r)

		// QR code routes (generate, validate, public lookup)
		controllers.RegisterQRRoutes(r)

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
