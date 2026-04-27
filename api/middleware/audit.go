// audit.go — API request audit logging middleware
// Logs every request to api_requests table for compliance tracking

package middleware

import (
	"github.com/go-chi/chi/v5/middleware"
	"log/slog"
	"net/http"
	"time"
)

// AuditService abstracts the audit log backend
type AuditService interface {
	LogRequest(
		actorID string,
		method string,
		path string,
		statusCode int,
		duration time.Duration,
		details map[string]string,
	) error
}

// AuditLogger logs all API requests
func AuditLogger(auditService AuditService) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()
			requestID := middleware.GetReqID(r.Context())
			claims := ClaimsFromContext(r.Context())

			// Get actor ID from claims
			actorID := "anonymous"
			if claims != nil && claims.Subject != "" {
				actorID = claims.Subject
			}

			// Wrap response writer to capture status code
			wrapped := middleware.NewWrapResponseWriter(w, r.ProtoMajor)

			// Call next handler
			next.ServeHTTP(wrapped, r)

			// Log the request
			duration := time.Since(start)
			details := map[string]string{
				"request_id": requestID,
				"remote_ip":  r.RemoteAddr,
				"user_agent": r.UserAgent(),
				"role":       "unknown",
			}
			if claims != nil {
				details["role"] = claims.Role
			}

			if err := auditService.LogRequest(
				actorID,
				r.Method,
				r.RequestURI,
				wrapped.Status(),
				duration,
				details,
			); err != nil {
				slog.Warn("audit log failed", "error", err)
				// Don't fail the request, just warn
			}

			slog.Info("api_request",
				"actor_id", actorID,
				"method", r.Method,
				"path", r.RequestURI,
				"status", wrapped.Status(),
				"duration_ms", duration.Milliseconds(),
				"request_id", requestID,
			)
		})
	}
}
