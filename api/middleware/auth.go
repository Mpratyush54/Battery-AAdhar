// auth.go — JWT validation middleware
// Extracts and validates RS256 tokens from Authorization header.
// On Day 3: validation stub. On Day 15: full RS256 + expiry check.

package middleware

import (
	"context"
	"fmt"
	"log/slog"
	"net/http"
	"strings"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// Claims holds parsed JWT payload
type Claims struct {
	Subject       string   `json:"sub"`       // user/manufacturer ID
	Role          string   `json:"role"`      // MANUFACTURER, REGULATOR, etc.
	ManufacturerID string   `json:"mfr_id,omitempty"`
	Permissions   []string `json:"perms,omitempty"`
	ExpiresAt     time.Time `json:"exp"`
}

// contextKey prevents collisions
type contextKey string

const claimsContextKey contextKey = "jwt_claims"

// Authenticate parses JWT from Authorization header and attaches Claims to context.
//
// Stub for Day 3: parses header but does NOT verify RS256 signature.
// On Day 15: full signature verification via public key.
func Authenticate(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Default: unauthenticated guest
		claims := &Claims{
			Role: "public",
		}

		authHeader := r.Header.Get("Authorization")
		if strings.HasPrefix(authHeader, "Bearer ") {
			tokenString := strings.TrimPrefix(authHeader, "Bearer ")
			if tokenString != "" {
				// TODO Day 15: Verify RS256 signature using public key from JWK endpoint
				// For now, just parse claims without verification
				parsedClaims, err := parseJWTClaims(tokenString)
				if err != nil {
					slog.Warn("invalid JWT", "error", err)
					// Fall through to guest claims
				} else {
					claims = parsedClaims
				}
			}
		}

		// Attach claims to request context
		ctx := context.WithValue(r.Context(), claimsContextKey, claims)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// parseJWTClaims parses JWT without signature verification (Day 3 stub).
func parseJWTClaims(tokenString string) (*Claims, error) {
	// This is a stub implementation that does NOT verify the signature.
	// It only parses the payload.
	token, _, err := new(jwt.Parser).ParseUnverified(tokenString, &jwt.MapClaims{})
	if err != nil {
		return nil, fmt.Errorf("parse JWT: %w", err)
	}

	mapClaims, ok := token.Claims.(jwt.MapClaims)
	if !ok {
		return nil, fmt.Errorf("invalid claims format")
	}

	// Extract fields
	claims := &Claims{
		Role: "authenticated", // default for any valid token
	}

	if sub, ok := mapClaims["sub"].(string); ok {
		claims.Subject = sub
	}

	if role, ok := mapClaims["role"].(string); ok {
		claims.Role = role
	}

	if mfrID, ok := mapClaims["mfr_id"].(string); ok {
		claims.ManufacturerID = mfrID
	}

	// Parse expiry (exp is a NumericDate: seconds since Unix epoch)
	if exp, ok := mapClaims["exp"].(float64); ok {
		claims.ExpiresAt = time.Unix(int64(exp), 0)
	}

	return claims, nil
}

// ClaimsFromContext retrieves JWT claims from request context.
func ClaimsFromContext(ctx context.Context) *Claims {
	v, _ := ctx.Value(claimsContextKey).(*Claims)
	if v == nil {
		// Return default guest claims if not found
		return &Claims{Role: "public"}
	}
	return v
}

// IsExpired checks if the JWT has expired.
func (c *Claims) IsExpired() bool {
	return time.Now().After(c.ExpiresAt)
}
