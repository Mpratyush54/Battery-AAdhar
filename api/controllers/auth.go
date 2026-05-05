package controllers

import (
	"encoding/json"
	"io"
	"log"
	"net/http"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
)

var authService = services.NewAuthService()

// RegisterStakeholderController godoc
// @Summary Register a new stakeholder
// @Description Registers a new stakeholder with Email and Password
// @Tags auth
// @Accept json
// @Produce json
// @Param payload body models.RegisterStakeholderPayload true "Registration payload"
// @Success 200 {object} models.RegisterStakeholderResponseJSON "Successful registration"
// @Failure 400 {string} string "Invalid payload"
// @Failure 500 {string} string "Internal Server Error"
// @Router /auth/register [post]
func RegisterStakeholderController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	bodyBuffer, err := io.ReadAll(req.Body)
	if err != nil {
		http.Error(res, "Error reading body", http.StatusInternalServerError)
		return
	}

	var payload models.RegisterStakeholderPayload
	if err := json.Unmarshal(bodyBuffer, &payload); err != nil {
		http.Error(res, "Invalid payload", http.StatusBadRequest)
		return
	}

	stakeholderID, err := authService.Register(
		payload.Email,
		payload.Password,
		payload.Role,
		payload.ProfileDetails,
		payload.AadharNumber,
		payload.AadharDocumentBase64,
	)

	if err != nil {
		if err == services.ErrUserExists {
			http.Error(res, err.Error(), http.StatusConflict)
			return
		}
		log.Printf("Registration error: %v", err)
		http.Error(res, "Registration failed: "+err.Error(), http.StatusInternalServerError)
		return
	}

	jsonResponse := models.RegisterStakeholderResponseJSON{
		StakeholderID: stakeholderID,
		Status:        "SUCCESS",
	}

	res.Header().Set("Content-Type", "application/json")
	json.NewEncoder(res).Encode(jsonResponse)
}

// LoginController godoc
// @Summary Login stakeholder
// @Description Logs in a stakeholder and sets HttpOnly cookies
// @Tags auth
// @Accept json
// @Produce json
// @Param payload body models.LoginPayload true "Login payload"
// @Success 200 {object} models.LoginResponseJSON "Successful login"
// @Failure 401 {string} string "Invalid credentials"
// @Router /auth/login [post]
func LoginController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	bodyBuffer, err := io.ReadAll(req.Body)
	if err != nil {
		http.Error(res, "Error reading request body", http.StatusInternalServerError)
		return
	}

	var payload models.LoginPayload
	if err := json.Unmarshal(bodyBuffer, &payload); err != nil {
		http.Error(res, "Invalid payload", http.StatusBadRequest)
		return
	}

	accessToken, refreshToken, stakeholderID, role, err := authService.Login(payload.Email, payload.Password)
	if err != nil {
		http.Error(res, "Authentication failed: "+err.Error(), http.StatusUnauthorized)
		return
	}

	// Set Access Token HTTP-Only Cookie
	http.SetCookie(res, &http.Cookie{
		Name:     "access_token",
		Value:    accessToken,
		HttpOnly: true,
		Secure:   true,
		Path:     "/",
		SameSite: http.SameSiteStrictMode,
		MaxAge:   900, // 15 minutes
	})

	// Set Refresh Token HTTP-Only Cookie
	http.SetCookie(res, &http.Cookie{
		Name:     "refresh_token",
		Value:    refreshToken,
		HttpOnly: true,
		Secure:   true,
		Path:     "/api/v1/auth/refresh",
		SameSite: http.SameSiteStrictMode,
		MaxAge:   604800, // 7 days
	})

	jsonResponse := models.LoginResponseJSON{
		StakeholderID: stakeholderID,
		Role:          role,
		Message:       "Logged in successfully",
	}

	res.Header().Set("Content-Type", "application/json")
	json.NewEncoder(res).Encode(jsonResponse)
}

// RefreshController godoc
// @Summary Refresh access token
// @Description Uses the refresh token cookie to get a new access token
// @Tags auth
// @Produce json
// @Success 200 {object} models.RefreshResponseJSON "Tokens refreshed"
// @Failure 401 {string} string "Invalid or missing refresh token"
// @Router /auth/refresh [post]
func RefreshController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	cookie, err := req.Cookie("refresh_token")
	if err != nil {
		http.Error(res, "Missing refresh token cookie", http.StatusUnauthorized)
		return
	}

	accessToken, refreshToken, err := authService.Refresh(cookie.Value)
	if err != nil {
		http.Error(res, "Failed to refresh token: "+err.Error(), http.StatusUnauthorized)
		return
	}

	http.SetCookie(res, &http.Cookie{
		Name:     "access_token",
		Value:    accessToken,
		HttpOnly: true,
		Secure:   true,
		Path:     "/",
		SameSite: http.SameSiteStrictMode,
		MaxAge:   900, // 15 minutes
	})

	if refreshToken != "" {
		http.SetCookie(res, &http.Cookie{
			Name:     "refresh_token",
			Value:    refreshToken,
			HttpOnly: true,
			Secure:   true,
			Path:     "/api/v1/auth/refresh",
			SameSite: http.SameSiteStrictMode,
			MaxAge:   604800, // 7 days
		})
	}

	jsonResponse := models.RefreshResponseJSON{
		Message: "Token refreshed successfully",
	}
	res.Header().Set("Content-Type", "application/json")
	json.NewEncoder(res).Encode(jsonResponse)
}

// LogoutController godoc
// @Summary Logout stakeholder
// @Description Clears the authentication cookies
// @Tags auth
// @Produce json
// @Success 200 {string} string "Logged out"
// @Router /auth/logout [post]
func LogoutController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	http.SetCookie(res, &http.Cookie{
		Name:     "access_token",
		Value:    "",
		HttpOnly: true,
		Secure:   true,
		Path:     "/",
		SameSite: http.SameSiteStrictMode,
		MaxAge:   -1,
	})

	http.SetCookie(res, &http.Cookie{
		Name:     "refresh_token",
		Value:    "",
		HttpOnly: true,
		Secure:   true,
		Path:     "/api/v1/auth/refresh",
		SameSite: http.SameSiteStrictMode,
		MaxAge:   -1,
	})

	res.Header().Set("Content-Type", "application/json")
	res.Write([]byte(`{"message":"Logged out successfully"}`))
}
