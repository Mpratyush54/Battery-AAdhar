// routes.go — chi-based router for the Battery Aadhaar API
// Replaces the previous http.ServeMux implementation.
// All existing route paths are preserved; only the router type changes.
package routes

import (
	"net/http"

	"github.com/go-chi/chi/v5"
	chiMiddleware "github.com/go-chi/chi/v5/middleware"
	httpSwagger "github.com/swaggo/http-swagger"

	"api/controllers"
	"api/middleware"
	// Import your existing handlers — adjust import paths as needed
)

// NewRouter constructs and returns the application chi.Router.
// All middleware is applied here in the correct order:
//   1. chi built-ins (request ID, real IP, recoverer)
//   2. custom logging   (structured zap/slog output)
//   3. custom auth      (JWT parse + attach claims to context)
//   4. custom RBAC      (role enforcement per route group)
func NewRouter() http.Handler {
	r := chi.NewRouter()

	// ── Global middleware (runs on every request) ─────────────────────────
	r.Use(chiMiddleware.RequestID)
	r.Use(chiMiddleware.RealIP)
	r.Use(chiMiddleware.Recoverer)
	r.Use(middleware.Logger)      // structured logging stub
	r.Use(middleware.Authenticate) // JWT parse — does NOT reject; just attaches claims

	// ── Health / readiness ────────────────────────────────────────────────
	r.Get("/healthz", handleHealthz)
	r.Get("/readyz",  handleReadyz)

	// ── Swagger UI ────────────────────────────────────────────────────────
	r.Get("/swagger/*", httpSwagger.Handler())

	// ── API v1 ────────────────────────────────────────────────────────────
	r.Route("/api/v1", func(r chi.Router) {

		// Auth endpoints
		r.Route("/auth", func(r chi.Router) {
			r.Post("/register", controllers.RegisterStakeholderController)
			r.Post("/login",    controllers.LoginController)
			r.Post("/refresh",  controllers.RefreshController)
			r.Post("/logout",   controllers.LogoutController)
		})

		// Public endpoints — no auth required beyond claim parse
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("public"))
			r.Get("/battery", controllers.GetBatteryController)
			r.Post("/batteries/scan",  handleScanQR)
		})

		// Authenticated manufacturer endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("manufacturer"))
			r.Post("/battery/register",       controllers.RegisterBatteryController)
			r.Get("/batteries/{bpan}/qr",     handleGetQR)
		})

		// Service provider / recycler endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("service_provider"))
			r.Get("/batteries/{bpan}/private",          handleGetPrivateData)
			r.Patch("/batteries/{bpan}/status",         handleUpdateStatus)
		})

		// Compliance / ZK verification endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("verifier"))
			r.Post("/batteries/{bpan}/verify/operational", handleVerifyOperational)
			r.Post("/batteries/{bpan}/verify/recyclable",  handleVerifyRecyclable)
			r.Post("/batteries/{bpan}/verify/signature",   handleVerifySignature)
		})

		// Admin-only
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("admin"))
			r.Post("/manufacturers",       handleRegisterManufacturer)
			r.Get("/manufacturers",        handleListManufacturers)
		})
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

func handleScanQR(w http.ResponseWriter, _ *http.Request)              { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleGetQR(w http.ResponseWriter, _ *http.Request)               { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleGetPrivateData(w http.ResponseWriter, _ *http.Request)      { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleUpdateStatus(w http.ResponseWriter, _ *http.Request)        { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleVerifyOperational(w http.ResponseWriter, _ *http.Request)   { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleVerifyRecyclable(w http.ResponseWriter, _ *http.Request)    { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleVerifySignature(w http.ResponseWriter, _ *http.Request)     { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleRegisterManufacturer(w http.ResponseWriter, _ *http.Request){ http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleListManufacturers(w http.ResponseWriter, _ *http.Request)   { http.Error(w, "not implemented", http.StatusNotImplemented) }
