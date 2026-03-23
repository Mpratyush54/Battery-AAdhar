package tests

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"testing"

	"api/controllers"
)

func TestBatteryEndpoints(t *testing.T) {
	tests := []struct {
		name           string
		method         string
		url            string
		body           []byte
		expectedStatus int
	}{
		{
			name:           "Register Battery Failure - Missing Service",
			method:         http.MethodPost,
			url:            "/api/v1/battery/register",
			body:           []byte(`{"manufacturerId":"6c9a3b66-1c88-444a-bea7-9e4b6b6537eb", "batteryCategory":"EV", "chemistryType":"LFP"}`),
			expectedStatus: http.StatusInternalServerError, // Returns 500 when grpc isn't mock injected
		},
		{
			name:           "Invalid Method",
			method:         http.MethodGet,
			url:            "/api/v1/battery/register",
			body:           nil,
			expectedStatus: http.StatusMethodNotAllowed,
		},
		{
			name:           "Bad Payload",
			method:         http.MethodPost,
			url:            "/api/v1/battery/register",
			body:           []byte(`{invalid_json}`),
			expectedStatus: http.StatusBadRequest,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			req, err := http.NewRequest(tc.method, tc.url, bytes.NewBuffer(tc.body))
			if err != nil {
				t.Fatal(err)
			}

			rr := httptest.NewRecorder()
			
			// Invoke the controller directly
			controllers.RegisterBatteryController(rr, req)

			if status := rr.Code; status != tc.expectedStatus {
				t.Errorf("handler returned wrong status code: got %v want %v",
					status, tc.expectedStatus)
			}
		})
	}
}
