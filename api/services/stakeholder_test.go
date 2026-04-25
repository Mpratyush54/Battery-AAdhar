package services

import (
	"context"
	"errors"
	"testing"

	cryptov1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/crypto/v1"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"google.golang.org/grpc"
)

// ── Registration Tests ────────────────────────────────────────────────────

func TestStakeholderService_RegisterStakeholder(t *testing.T) {
	mockClient := &mockCryptoClient{}
	encService := NewEncryptionService(mockClient)
	service := NewStakeholderService(encService)

	req := &models.StakeholderRegistration{
		StakeholderType:  models.TypeManufacturer,
		OrganizationName: "Test Org",
		CountryCode:      "IN",
		ContactEmail:     "test@org.com",
		ContactPhone:     "1234567890",
		AadhaarDocument:  []byte("secret-kyc-doc"),
	}

	stakeholder, err := service.RegisterStakeholder(context.Background(), req)
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if stakeholder.OrganizationName != "Test Org" {
		t.Errorf("expected Test Org, got %s", stakeholder.OrganizationName)
	}
	if string(stakeholder.AadhaarEncrypted) != "mock-ciphertext" {
		t.Errorf("expected mock-ciphertext, got %s", string(stakeholder.AadhaarEncrypted))
	}
	if stakeholder.Status != "pending" {
		t.Errorf("expected pending status, got %s", stakeholder.Status)
	}
	if stakeholder.ID.String() == "" {
		t.Error("expected non-empty stakeholder ID")
	}
}

func TestStakeholderService_RegisterStakeholder_AllTypes(t *testing.T) {
	mockClient := &mockCryptoClient{}
	encService := NewEncryptionService(mockClient)
	service := NewStakeholderService(encService)

	types := []models.StakeholderType{
		models.TypeManufacturer,
		models.TypeRecycler,
		models.TypeGovernment,
	}

	for _, st := range types {
		t.Run(string(st), func(t *testing.T) {
			req := &models.StakeholderRegistration{
				StakeholderType:  st,
				OrganizationName: "Org-" + string(st),
				CountryCode:      "IN",
				AadhaarDocument:  []byte("doc"),
			}
			stakeholder, err := service.RegisterStakeholder(context.Background(), req)
			if err != nil {
				t.Fatalf("expected no error for type %s, got %v", st, err)
			}
			if stakeholder.StakeholderType != st {
				t.Errorf("expected type %s, got %s", st, stakeholder.StakeholderType)
			}
		})
	}
}

// ── Validation Tests ──────────────────────────────────────────────────────

func TestStakeholderService_RegisterStakeholder_EmptyOrgName(t *testing.T) {
	mockClient := &mockCryptoClient{}
	encService := NewEncryptionService(mockClient)
	service := NewStakeholderService(encService)

	req := &models.StakeholderRegistration{
		StakeholderType:  models.TypeManufacturer,
		OrganizationName: "",
		CountryCode:      "IN",
	}

	_, err := service.RegisterStakeholder(context.Background(), req)
	if err == nil {
		t.Fatal("expected error for empty organization name")
	}
}

func TestStakeholderService_RegisterStakeholder_InvalidCountryCode(t *testing.T) {
	mockClient := &mockCryptoClient{}
	encService := NewEncryptionService(mockClient)
	service := NewStakeholderService(encService)

	invalidCodes := []string{"", "I", "IND", "1234"}
	for _, code := range invalidCodes {
		t.Run("code_"+code, func(t *testing.T) {
			req := &models.StakeholderRegistration{
				StakeholderType:  models.TypeManufacturer,
				OrganizationName: "Test Org",
				CountryCode:      code,
			}
			_, err := service.RegisterStakeholder(context.Background(), req)
			if err == nil {
				t.Fatalf("expected error for country code %q", code)
			}
		})
	}
}

// ── Encryption Failure Propagation ────────────────────────────────────────

func TestStakeholderService_RegisterStakeholder_EncryptionFailure(t *testing.T) {
	mockClient := &mockCryptoClient{
		encryptFunc: func(ctx context.Context, in *cryptov1.EncryptRequest, opts ...grpc.CallOption) (*cryptov1.EncryptResponse, error) {
			return nil, errors.New("Rust crypto service unavailable")
		},
	}
	encService := NewEncryptionService(mockClient)
	service := NewStakeholderService(encService)

	req := &models.StakeholderRegistration{
		StakeholderType:  models.TypeManufacturer,
		OrganizationName: "Test Org",
		CountryCode:      "IN",
		AadhaarDocument:  []byte("secret-kyc-doc"),
	}

	_, err := service.RegisterStakeholder(context.Background(), req)
	if err == nil {
		t.Fatal("expected error when encryption service fails")
	}
}

// ── DecryptKYCDocument Tests ──────────────────────────────────────────────

func TestStakeholderService_DecryptKYCDocument(t *testing.T) {
	mockClient := &mockCryptoClient{}
	encService := NewEncryptionService(mockClient)
	service := NewStakeholderService(encService)

	plaintext, err := service.DecryptKYCDocument(
		context.Background(),
		"some-stakeholder-id",
		[]byte("encrypted-data"),
		1,
	)
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if string(plaintext) != "mock-plaintext" {
		t.Errorf("expected mock-plaintext, got %s", string(plaintext))
	}
}

func TestStakeholderService_DecryptKYCDocument_Failure(t *testing.T) {
	mockClient := &mockCryptoClient{
		decryptFunc: func(ctx context.Context, in *cryptov1.DecryptRequest, opts ...grpc.CallOption) (*cryptov1.DecryptResponse, error) {
			return nil, errors.New("decryption denied")
		},
	}
	encService := NewEncryptionService(mockClient)
	service := NewStakeholderService(encService)

	_, err := service.DecryptKYCDocument(
		context.Background(),
		"some-stakeholder-id",
		[]byte("encrypted-data"),
		1,
	)
	if err == nil {
		t.Fatal("expected error when decryption fails")
	}
}

// ── GetStakeholder Stub Test ──────────────────────────────────────────────

func TestStakeholderService_GetStakeholder_NotImplemented(t *testing.T) {
	mockClient := &mockCryptoClient{}
	encService := NewEncryptionService(mockClient)
	service := NewStakeholderService(encService)

	_, err := service.GetStakeholder(context.Background(), "some-id")
	if err == nil {
		t.Fatal("expected error for unimplemented method")
	}
}
