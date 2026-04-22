package tests

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/Mpratyush54/Battery-AAdhar/api/routes"
)

// TestSimulation verifies an end-to-end simulation of the HTTP routes, including how the RBAC middleware correctly filters and allows requests across different endpoints.
func TestSimulation(t *testing.T) {
	mux := routes.NewRouter()

	// Spin up a test HTTP server
	ts := httptest.NewServer(mux)
	defer ts.Close()

	client := &http.Client{}

	// --- 1. Test Auth Endpoints (No role required) ---
	t.Run("Auth Simulation", func(t *testing.T) {
		req, _ := http.NewRequest(http.MethodPost, ts.URL+"/api/v1/auth/login", bytes.NewBuffer([]byte(`{"email":"test","password":"test"}`)))
		resp, err := client.Do(req)
		if err != nil {
			t.Fatalf("Failed to make request: %v", err)
		}
		defer resp.Body.Close()
		// Will likely fail because DB isn't mocked but should hit 500/401 and NOT 404
		if resp.StatusCode == http.StatusNotFound {
			t.Error("Expected Auth route to exist, got 404")
		}
	})

	// Create a helper wrapper manually injecting JWT claims equivalent to a manufacturer for deep endpoint testing
	runRequestWithRoleMock := func(method, path string, userRole string) int {
		req, _ := http.NewRequest(method, ts.URL+path, nil)

		// Wait, we need to bypass Authenticate to inject a manual role into the request lifecycle for integration mock testing.
		// A cleaner way is using the normal chi router, but manually constructing a request directly against the mux
		// and inserting the context directly before ServeHTTP.
		req, _ = http.NewRequest(method, path, nil)

		// The custom router mounts Validate() at the top. We can't easily mock ctx via http client DO.
		// Instead we'll hit the server via httptest.Recorder with context overrides.
		rr := httptest.NewRecorder()

		// The router's Authenticate middleware will OVERWRITE our manual context if we don't supply a valid parsed token!
		// But remember: Authenticate currently just sets 'public' or 'authenticated' if there's a token!
		// For an end-to-end simulated test without RSA keys right now, we can check basic protections.
		if userRole != "public" {
			req.Header.Set("Authorization", "Bearer dummy-token")
			// Authenticate stub will currently hardcode role="authenticated"
		}

		mux.ServeHTTP(rr, req)
		resp := rr.Result()
		return resp.StatusCode
	}

	// --- 2. Test Battery Routes with Auth Header ---
	t.Run("Battery Manufacturer Protection Simulation", func(t *testing.T) {
		status := runRequestWithRoleMock(http.MethodPost, "/api/v1/battery/register", "authenticated")
		// Since stub auth gives "authenticated", and route requires "manufacturer", this MUST be 403 Forbidden
		if status != http.StatusForbidden {
			t.Errorf("Expected 403 Forbidden for insufficient role, got %d", status)
		}
	})

	t.Run("Public Route Simulation", func(t *testing.T) {
		status := runRequestWithRoleMock(http.MethodGet, "/api/v1/battery?bpan=123", "public")
		// Public role should pass RBAC. The controller will error 500 without Microservice.
		if status == http.StatusForbidden || status == http.StatusNotFound {
			t.Errorf("Expected route to be found and allowed (500 without DB), got %d", status)
		}
	})

	t.Run("Swagger UI exists", func(t *testing.T) {
		req, _ := http.NewRequest(http.MethodGet, ts.URL+"/swagger/index.html", nil)
		resp, _ := client.Do(req)
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusOK {
			t.Errorf("Expected 200 for Swagger UI, got %d", resp.StatusCode)
		}
	})
}
