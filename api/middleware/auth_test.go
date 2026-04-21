package middleware

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestAuthenticate(t *testing.T) {
	// A dummy handler to test if claims are attached properly
	nextHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		claims := ClaimsFromContext(r.Context())
		if claims == nil {
			t.Fatal("Expected claims in context, got nil")
		}

		authHeader := r.Header.Get("Authorization")
		if authHeader == "" {
			if claims.Role != "public" {
				t.Errorf("Expected 'public' role for empty auth header, got %s", claims.Role)
			}
		} else {
			if claims.Role != "authenticated" {
				t.Errorf("Expected 'authenticated' role for stub valid token, got %s", claims.Role)
			}
		}
		w.WriteHeader(http.StatusOK)
	})

	handler := Authenticate(nextHandler)

	t.Run("No Authorization Header", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodGet, "/", nil)
		rr := httptest.NewRecorder()

		handler.ServeHTTP(rr, req)
		if rr.Code != http.StatusOK {
			t.Errorf("Expected status 200, got %d", rr.Code)
		}
	})

	t.Run("With Authorization Header", func(t *testing.T) {
		req := httptest.NewRequest(http.MethodGet, "/", nil)
		req.Header.Set("Authorization", "Bearer fake-jwt-token")
		rr := httptest.NewRecorder()

		handler.ServeHTTP(rr, req)
		if rr.Code != http.StatusOK {
			t.Errorf("Expected status 200, got %d", rr.Code)
		}
	})
}
