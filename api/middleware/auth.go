// auth.go — JWT validation middleware
// Extracts and validates HS256 tokens from Authorization header.
// On Day 3: expiry check added. On Day 22: migrate to RS256 + signature verification.

package middleware

import (
	"context"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// Claims holds parsed JWT payload
type Claims struct {
	Subject        string    `json:"sub"`  // user/manufacturer ID
	Role           string    `json:"role"` // MANUFACTURER, REGULATOR, etc.
	ManufacturerID string    `json:"mfr_id,omitempty"`
	Permissions    []string  `json:"perms,omitempty"`
	ExpiresAt      time.Time `json:"exp"`
}

// contextKey prevents collisions
type contextKey string

const claimsContextKey contextKey = "jwt_claims"

// Authenticate parses JWT from Authorization header, validates expiry,
// and attaches Claims to context. Rejects expired or malformed tokens.
//
// Day 3: HS256 with expiry check.
// Day 22: migrate to RS256 signature verification.
func Authenticate(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		authHeader := r.Header.Get("Authorization")
		if !strings.HasPrefix(authHeader, "Bearer ") {
			// No token — attach guest claims and continue
			ctx := context.WithValue(r.Context(), claimsContextKey, &Claims{Role: "public"})
			next.ServeHTTP(w, r.WithContext(ctx))
			return
		}

		tokenString := strings.TrimPrefix(authHeader, "Bearer ")
		if tokenString == "" {
			ctx := context.WithValue(r.Context(), claimsContextKey, &Claims{Role: "public"})
			next.ServeHTTP(w, r.WithContext(ctx))
			return
		}

		claims, err := parseAndValidateJWT(tokenString)
		if err != nil {
			slog.Warn("invalid JWT", "error", err)
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusUnauthorized)
			jsonError(w, "invalid or expired token")
			return
		}

		// Attach validated claims to request context
		ctx := context.WithValue(r.Context(), claimsContextKey, claims)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// parseAndValidateJWT parses and validates an HS256 JWT.
// Checks: signature (HS256), expiry, and required claims.
func parseAndValidateJWT(tokenString string) (*Claims, error) {
	secret := os.Getenv("JWT_SECRET")
	if secret == "" {
		secret = "fallback_secret_key" // Dev only — must be overridden in production
	}

	token, err := jwt.Parse(tokenString, func(token *jwt.Token) (interface{}, error) {
		// Validate signing method
		if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("unexpected signing method: %v", token.Header["alg"])
		}
		return []byte(secret), nil
	}, jwt.WithValidMethods([]string{"HS256"}))

	if err != nil {
		return nil, fmt.Errorf("parse JWT: %w", err)
	}

	mapClaims, ok := token.Claims.(jwt.MapClaims)
	if !ok {
		return nil, fmt.Errorf("invalid claims format")
	}

	// Extract fields
	claims := &Claims{
		Role: "authenticated",
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

	// Parse expiry
	if exp, ok := mapClaims["exp"].(float64); ok {
		claims.ExpiresAt = time.Unix(int64(exp), 0)
		if time.Now().After(claims.ExpiresAt) {
			return nil, fmt.Errorf("token expired at %v", claims.ExpiresAt)
		}
	} else {
		return nil, fmt.Errorf("token missing expiry claim")
	}

	return claims, nil
}

// ClaimsFromContext retrieves JWT claims from request context.
func ClaimsFromContext(ctx context.Context) *Claims {
	v, _ := ctx.Value(claimsContextKey).(*Claims)
	if v == nil {
		return &Claims{Role: "public"}
	}
	return v
}

// GetUserID returns the authenticated user's subject (ID) from the request context.
func GetUserID(r *http.Request) string {
	return ClaimsFromContext(r.Context()).Subject
}

// GetUserRole returns the authenticated user's role from the request context.
func GetUserRole(r *http.Request) string {
	return ClaimsFromContext(r.Context()).Role
}

// IsExpired checks if the JWT has expired.
func (c *Claims) IsExpired() bool {
	return time.Now().After(c.ExpiresAt)
}

func jsonError(w http.ResponseWriter, msg string) {
	w.Write([]byte(`{"error":"` + msg + `"}`))
}
