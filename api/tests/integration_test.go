package tests

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"testing"

	"api/routes"
)

// TestFullAppEndpoints runs integration tests over the actual configured HTTP Router
// verifying that handlers map correctly to their HTTP methods and return proper HTTP headers.
func TestFullAppEndpoints(t *testing.T) {
	mux := routes.SetupRoutes()

	// Spin up a test HTTP server
	ts := httptest.NewServer(mux)
	defer ts.Close()

	tests := []struct {
		name           string
		method         string
		url            string
		body           []byte
		expectedStatus int
	}{
		{
			name:           "Valid Registration Route Exists",
			method:         http.MethodPost,
			url:            ts.URL + "/api/v1/auth/register",
			body:           []byte(`{"email":"test@test.com", "password":"test", "role":"End-user"}`),
			expectedStatus: http.StatusInternalServerError, // Fails gracefully 500 because gRPC backend is detached in this test
		},
		{
			name:           "Invalid Method For Registration",
			method:         http.MethodGet,
			url:            ts.URL + "/api/v1/auth/register",
			body:           nil,
			expectedStatus: http.StatusMethodNotAllowed,
		},
		{
			name:           "Valid Battery Register Route",
			method:         http.MethodPost,
			url:            ts.URL + "/api/v1/battery/register",
			body:           []byte(`{"manufacturerId":"6c9a3b66-1c88-444a-bea7-9e4b6b6537eb", "batteryCategory":"EV"}`),
			expectedStatus: http.StatusInternalServerError,
		},
		{
			name:           "Valid Get Battery Route",
			method:         http.MethodGet,
			url:            ts.URL + "/api/v1/battery?bpan=123",
			body:           nil,
			expectedStatus: http.StatusInternalServerError, // Fails 500 without backend mock
		},
		{
			name:           "Invalid Method For Battery",
			method:         http.MethodPost,
			url:            ts.URL + "/api/v1/battery",
			body:           nil,
			expectedStatus: http.StatusMethodNotAllowed,
		},
		{
			name:           "Swagger Route Accessible",
			method:         http.MethodGet,
			url:            ts.URL + "/swagger/index.html",
			body:           nil,
			expectedStatus: http.StatusOK,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			req, err := http.NewRequest(tc.method, tc.url, bytes.NewBuffer(tc.body))
			if err != nil {
				t.Fatalf("Failed to construct request: %v", err)
			}

			client := &http.Client{}
			resp, err := client.Do(req)
			if err != nil {
				t.Fatalf("Failed to execute request: %v", err)
			}
			defer resp.Body.Close()

			if status := resp.StatusCode; status != tc.expectedStatus {
				t.Errorf("expected status %v; got %v", tc.expectedStatus, status)
			}
		})
	}
}
