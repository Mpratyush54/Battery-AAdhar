// routes.go — chi-based router for the Battery Aadhaar API
// Replaces the previous http.ServeMux implementation.
// All existing route paths are preserved; only the router type changes.
package routes

import (
	"net/http"

	"github.com/go-chi/chi/v5"
	chiMiddleware "github.com/go-chi/chi/v5/middleware"
	httpSwagger "github.com/swaggo/http-swagger"

	"github.com/Mpratyush54/Battery-AAdhar/api/controllers"
	_ "github.com/Mpratyush54/Battery-AAdhar/api/docs"
	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

// NewRouter constructs and returns the application chi.Router.
// All middleware is applied here in the correct order:
//  1. chi built-ins (request ID, real IP, recoverer)
//  2. custom logging   (structured zap/slog output)
//  3. custom auth      (JWT parse + attach claims to context)
//  4. custom RBAC      (role enforcement per route group)
func NewRouter() http.Handler {
	r := chi.NewRouter()

	// ── Global middleware (runs on every request) ─────────────────────────
	r.Use(chiMiddleware.RequestID)
	r.Use(chiMiddleware.RealIP)
	r.Use(chiMiddleware.Recoverer)
	r.Use(middleware.Logger)       // structured logging stub
	r.Use(middleware.Authenticate) // JWT parse — does NOT reject; just attaches claims

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

		// Authenticated manufacturer endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireResource(models.ResourceBattery, models.ActionCreate))
			r.Post("/battery/register", controllers.RegisterBatteryController)
		})

		// Service provider endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireResource(models.ResourceBatteryHealth, models.ActionUpdate))
			r.Patch("/batteries/{bpan}/status", handleUpdateStatus)
		})

		// Compliance / ZK verification endpoints (verifier role)
		r.Group(func(r chi.Router) {
			r.Use(middleware.IsRole("verifier"))
			r.Post("/batteries/{bpan}/verify/operational", handleVerifyOperational)
			r.Post("/batteries/{bpan}/verify/signature", handleVerifySignature)
		})

		// Admin-only
		r.Group(func(r chi.Router) {
			r.Use(middleware.IsRole("admin"))
			r.Post("/manufacturers", handleRegisterManufacturer)
			r.Get("/manufacturers", handleListManufacturers)
		})

		// ── Controller-based routes (each controller handles its own RBAC) ──
		controllers.RegisterMaterialRoutes(r)
		controllers.RegisterHealthRoutes(r)
		controllers.RegisterLifecycleRoutes(r)
		controllers.RegisterComplianceRoutes(r)
		controllers.RegisterTelemetryRoutes(r)
		controllers.RegisterQRRoutes(r)
	})

	return r
}

// ── Placeholder handlers (replace with real handlers as they are built) ──

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
func handleVerifyOperational(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}
func handleVerifySignature(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}
func handleRegisterManufacturer(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}
func handleListManufacturers(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}
