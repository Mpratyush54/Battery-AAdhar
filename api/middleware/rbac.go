// rbac.go — role-based access control middleware
// Role hierarchy (lowest → highest privilege):
//   public < authenticated < manufacturer < service_provider < recycler < verifier < government < admin
package middleware

import (
	"net/http"
	"slices"
)

// roleHierarchy defines the ordered privilege tiers.
// A role grants access to its own tier and all tiers below it.
var roleHierarchy = []string{
	"public",
	"authenticated",
	"manufacturer",
	"service_provider",
	"recycler",
	"verifier",
	"government",
	"admin",
}

func roleLevel(role string) int {
	idx := slices.Index(roleHierarchy, role)
	if idx < 0 {
		return -1 // unknown role → no access
	}
	return idx
}

// RequireRole returns a middleware that rejects requests whose JWT role
// is below the required level.
func RequireRole(required string) func(http.Handler) http.Handler {
	requiredLevel := roleLevel(required)

	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			claims := ClaimsFromContext(r.Context())
			if requiredLevel < 0 || claims == nil || roleLevel(claims.Role) < requiredLevel {
				http.Error(w,
					`{"error":"insufficient_role","required":"`+required+`"}`,
					http.StatusForbidden,
				)
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}
