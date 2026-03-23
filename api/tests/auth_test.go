package tests

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"testing"

	"api/controllers"
)

func TestAuthEndpoints(t *testing.T) {
	tests := []struct {
		name           string
		method         string
		url            string
		body           []byte
		expectedStatus int
	}{
		{
			name:           "Register Failure - Missing Service",
			method:         http.MethodPost,
			url:            "/api/v1/auth/register",
			body:           []byte(`{"email":"test@test.com", "password":"test", "role":"End-user"}`),
			expectedStatus: http.StatusInternalServerError, // Fails because gRPC isn't mock injected in this simple test
		},
		{
			name:           "Login Failure - Missing Service",
			method:         http.MethodPost,
			url:            "/api/v1/auth/login",
			body:           []byte(`{"email":"test@test.com", "password":"test"}`),
			expectedStatus: http.StatusInternalServerError,
		},
		{
			name:           "Refresh Token Missing Cookie",
			method:         http.MethodPost,
			url:            "/api/v1/auth/refresh",
			body:           nil,
			expectedStatus: http.StatusUnauthorized,
		},
		{
			name:           "Logout Success",
			method:         http.MethodPost,
			url:            "/api/v1/auth/logout",
			body:           nil,
			expectedStatus: http.StatusOK,
		},
		{
			name:           "Invalid Method",
			method:         http.MethodGet,
			url:            "/api/v1/auth/login",
			body:           nil,
			expectedStatus: http.StatusMethodNotAllowed,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			req, err := http.NewRequest(tc.method, tc.url, bytes.NewBuffer(tc.body))
			if err != nil {
				t.Fatal(err)
			}

			rr := httptest.NewRecorder()
			// Manually route it
			switch tc.url {
			case "/api/v1/auth/register":
				controllers.RegisterStakeholderController(rr, req)
			case "/api/v1/auth/login":
				controllers.LoginController(rr, req)
			case "/api/v1/auth/refresh":
				controllers.RefreshController(rr, req)
			case "/api/v1/auth/logout":
				controllers.LogoutController(rr, req)
			}

			if status := rr.Code; status != tc.expectedStatus {
				// We expect some 500s because grpc service isn't wired in unit tests
				if status != tc.expectedStatus {
					t.Errorf("handler returned wrong status code: got %v want %v",
						status, tc.expectedStatus)
				}
			}
		})
	}
}
