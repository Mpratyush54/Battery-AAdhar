package services

import (
	"crypto/rand"
	"database/sql"
	"encoding/base64"
	"errors"
	"os"
	"time"

	"github.com/Mpratyush54/Battery-AAdhar/api/config"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"golang.org/x/crypto/bcrypt"
)

var (
	ErrUserExists   = errors.New("user already exists with this email")
	ErrInvalidCreds = errors.New("invalid credentials")
	ErrInvalidToken = errors.New("invalid or expired token")
)

type AuthService struct{}

func NewAuthService() *AuthService {
	return &AuthService{}
}

func (s *AuthService) Register(email, password, role, profileDetails, aadharNumber, aadharDocumentBase64 string) (string, error) {
	var count int
	err := config.DB.QueryRow("SELECT COUNT(*) FROM stakeholder_credentials WHERE email = $1", email).Scan(&count)
	if err != nil {
		return "", err
	}
	if count > 0 {
		return "", ErrUserExists
	}

	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return "", err
	}

	tx, err := config.DB.Begin()
	if err != nil {
		return "", err
	}
	defer tx.Rollback()

	stakeholderID := uuid.New().String()

	_, err = tx.Exec(
		"INSERT INTO stakeholders (id, role, encrypted_profile) VALUES ($1, $2, $3)",
		stakeholderID, role, profileDetails, // Not encrypting natively in Go for now, wait, user said "only thing for things when data of battery comes so it needs to encrypted that will be done by rust"
	)
	if err != nil {
		return "", err
	}

	_, err = tx.Exec(
		"INSERT INTO stakeholder_credentials (stakeholder_id, email, password_hash) VALUES ($1, $2, $3)",
		stakeholderID, email, hashedPassword,
	)
	if err != nil {
		return "", err
	}

	_, err = tx.Exec(
		"INSERT INTO stakeholder_kyc (stakeholder_id, aadhar_number, aadhar_document_base64) VALUES ($1, $2, $3)",
		stakeholderID, aadharNumber, aadharDocumentBase64,
	)
	if err != nil {
		return "", err
	}

	if err := tx.Commit(); err != nil {
		return "", err
	}

	return stakeholderID, nil
}

func (s *AuthService) Login(email, password string) (string, string, string, string, error) {
	var stakeholderID, hash, role string

	err := config.DB.QueryRow(`
		SELECT c.stakeholder_id, c.password_hash, s.role 
		FROM stakeholder_credentials c
		JOIN stakeholders s ON s.id = c.stakeholder_id
		WHERE c.email = $1
	`, email).Scan(&stakeholderID, &hash, &role)

	if err != nil {
		if err == sql.ErrNoRows {
			return "", "", "", "", ErrInvalidCreds
		}
		return "", "", "", "", err
	}

	if err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(password)); err != nil {
		return "", "", "", "", ErrInvalidCreds
	}

	accessToken, err := generateJWT(stakeholderID, role)
	if err != nil {
		return "", "", "", "", err
	}

	refreshToken, err := generateRefreshToken(stakeholderID)
	if err != nil {
		return "", "", "", "", err
	}

	return accessToken, refreshToken, stakeholderID, role, nil
}

func (s *AuthService) Refresh(refreshToken string) (string, string, error) {
	var id, stakeholderID, role string
	var expiresAt time.Time

	err := config.DB.QueryRow(`
		SELECT r.id, r.stakeholder_id, r.expires_at, s.role 
		FROM refresh_tokens r
		JOIN stakeholders s ON s.id = r.stakeholder_id
		WHERE r.token = $1 AND r.revoked = FALSE
	`, refreshToken).Scan(&id, &stakeholderID, &expiresAt, &role)

	if err != nil || expiresAt.Before(time.Now()) {
		return "", "", ErrInvalidToken
	}

	_, err = config.DB.Exec("UPDATE refresh_tokens SET revoked = TRUE WHERE id = $1", id)
	if err != nil {
		return "", "", err
	}

	newAccessToken, err := generateJWT(stakeholderID, role)
	if err != nil {
		return "", "", err
	}

	newRefreshToken, err := generateRefreshToken(stakeholderID)
	if err != nil {
		return "", "", err
	}

	return newAccessToken, newRefreshToken, nil
}

func generateJWT(stakeholderID, role string) (string, error) {
	secret := os.Getenv("JWT_SECRET")
	if secret == "" {
		secret = "fallback_secret_key"
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, jwt.MapClaims{
		"sub":  stakeholderID,
		"role": role,
		"exp":  time.Now().Add(15 * time.Minute).Unix(),
	})

	return token.SignedString([]byte(secret))
}

func generateRefreshToken(stakeholderID string) (string, error) {
	b := make([]byte, 32)
	_, err := rand.Read(b)
	if err != nil {
		return "", err
	}

	token := base64.URLEncoding.EncodeToString(b)
	expiresAt := time.Now().AddDate(0, 0, 7) // 7 days

	_, err = config.DB.Exec(
		"INSERT INTO refresh_tokens (id, stakeholder_id, token, expires_at) VALUES ($1, $2, $3, $4)",
		uuid.New().String(), stakeholderID, token, expiresAt,
	)
	if err != nil {
		return "", err
	}

	return token, nil
}
