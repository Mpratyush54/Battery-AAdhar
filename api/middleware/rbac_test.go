package middleware

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestIsRole(t *testing.T) {
	// A dummy handler that represents a protected endpoint
	successHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	})

	tests := []struct {
		name         string
		requiredRole string
		userRole     string
		expectStatus int
	}{
		{"Public route accessed by public", "public", "public", http.StatusOK},
		{"Admin route accessed by admin", "admin", "admin", http.StatusOK},
		{"Public route accessed by admin", "public", "admin", http.StatusForbidden},
		{"Manufacturer route accessed by public", "manufacturer", "public", http.StatusForbidden},
		{"Manufacturer route accessed by manufacturer", "manufacturer", "manufacturer", http.StatusOK},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			handler := IsRole(tt.requiredRole)(successHandler)
			req := httptest.NewRequest(http.MethodGet, "/", nil)

			// Inject Claims into context
			claims := &Claims{Role: tt.userRole}
			ctx := context.WithValue(req.Context(), claimsContextKey, claims)
			req = req.WithContext(ctx)

			rr := httptest.NewRecorder()
			handler.ServeHTTP(rr, req)

			if rr.Code != tt.expectStatus {
				t.Errorf("Expected status %d, got %d", tt.expectStatus, rr.Code)
			}
		})
	}
}
