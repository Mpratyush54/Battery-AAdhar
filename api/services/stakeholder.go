// stakeholder.go — Stakeholder service (registration, KYC, etc.)

package services

import (
	"context"
	"fmt"
	"log/slog"

	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/google/uuid"
)

// StakeholderService handles all stakeholder operations
type StakeholderService struct {
	encryptionService *EncryptionService
}

// NewStakeholderService creates a new stakeholder service
func NewStakeholderService(encService *EncryptionService) *StakeholderService {
	return &StakeholderService{
		encryptionService: encService,
	}
}

// RegisterStakeholder registers a new stakeholder and encrypts their KYC document
func (s *StakeholderService) RegisterStakeholder(
	ctx context.Context,
	reg *models.StakeholderRegistration,
) (*models.Stakeholder, error) {
	if reg.OrganizationName == "" {
		return nil, fmt.Errorf("organization_name required")
	}
	if reg.CountryCode == "" || len(reg.CountryCode) != 2 {
		return nil, fmt.Errorf("valid country_code required")
	}

	stakeholder := &models.Stakeholder{
		ID:               uuid.New(),
		StakeholderType:  reg.StakeholderType,
		OrganizationName: reg.OrganizationName,
		CountryCode:      reg.CountryCode,
		ContactEmail:     reg.ContactEmail,
		ContactPhone:     reg.ContactPhone,
		Status:           "pending",
	}

	encrypted, err := s.encryptionService.Encrypt(
		ctx,
		stakeholder.ID.String(),
		"aadhaar_document",
		reg.AadhaarDocument,
	)
	if err != nil {
		slog.Error("failed to encrypt KYC", "error", err)
		return nil, fmt.Errorf("KYC encryption failed: %w", err)
	}

	stakeholder.AadhaarEncrypted = encrypted.Ciphertext

	slog.Info("stakeholder registered", "id", stakeholder.ID, "type", stakeholder.StakeholderType)

	return stakeholder, nil
}

// GetStakeholder retrieves a stakeholder by ID
func (s *StakeholderService) GetStakeholder(ctx context.Context, id string) (*models.Stakeholder, error) {
	slog.Info("get_stakeholder", "id", id)
	return nil, fmt.Errorf("stakeholder lookup requires DB wiring (Day 17+)")
}

// DecryptKYCDocument decrypts a stakeholder's KYC document
// Only authorized users (government, admin) can call this
func (s *StakeholderService) DecryptKYCDocument(
	ctx context.Context,
	stakeholderID string,
	encryptedData []byte,
	kekVersion int32,
) ([]byte, error) {
	plaintext, err := s.encryptionService.Decrypt(
		ctx,
		stakeholderID, // Use stakeholder ID as decryption context
		"aadhaar_document",
		encryptedData,
		kekVersion,
		"AES-256-GCM",
	)
	if err != nil {
		return nil, fmt.Errorf("KYC decryption failed: %w", err)
	}

	return plaintext, nil
}
