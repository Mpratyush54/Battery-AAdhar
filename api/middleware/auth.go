// auth.go — JWT parsing middleware (does NOT enforce; just attaches claims)
// Role enforcement is in rbac.go — RequireRole().
package middleware

import (
	"context"
	"net/http"
	"strings"
)

// contextKey is unexported to avoid collisions with other packages.
type contextKey string

const (
	claimsKey contextKey = "claims"
)

// Claims holds the parsed JWT payload for a request.
// Full JWT validation (RS256 signature, expiry) is added on Day 15.
type Claims struct {
	Subject        string   `json:"sub"`
	Role           string   `json:"role"`
	ManufacturerID string   `json:"manufacturer_id,omitempty"`
	Permissions    []string `json:"permissions,omitempty"`
}

// Authenticate parses the Authorization: Bearer <token> header and attaches
// Claims to the request context. Requests without a token get a guest/public
// Claims so downstream handlers always find a non-nil value.
//
// STUB: On Day 15 this will verify the RS256 signature and expiry.
// Today it only parses the header so the middleware chain compiles.
func Authenticate(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		claims := &Claims{Role: "public"} // default: unauthenticated guest

		authHeader := r.Header.Get("Authorization")
		if strings.HasPrefix(authHeader, "Bearer ") {
			token := strings.TrimPrefix(authHeader, "Bearer ")
			if token != "" {
				// TODO Day 15: verify RS256, parse sub/role from JWT
				// For now, just mark as authenticated with placeholder
				claims = &Claims{
					Subject: "stub-subject",
					Role:    "authenticated",
				}
			}
		}

		ctx := context.WithValue(r.Context(), claimsKey, claims)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// ClaimsFromContext retrieves the Claims attached by Authenticate.
// Returns nil if called before Authenticate runs (should not happen).
func ClaimsFromContext(ctx context.Context) *Claims {
	v, _ := ctx.Value(claimsKey).(*Claims)
	return v
}
