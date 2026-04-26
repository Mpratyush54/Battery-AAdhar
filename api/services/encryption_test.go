package services

import (
	"context"
	"errors"
	"testing"

	cryptov1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/crypto/v1"
	"google.golang.org/grpc"
)

// ── Mock gRPC client ──────────────────────────────────────────────────────

type mockCryptoClient struct {
	cryptov1.CryptoServiceClient
	encryptFunc func(ctx context.Context, in *cryptov1.EncryptRequest, opts ...grpc.CallOption) (*cryptov1.EncryptResponse, error)
	decryptFunc func(ctx context.Context, in *cryptov1.DecryptRequest, opts ...grpc.CallOption) (*cryptov1.DecryptResponse, error)
}

func (m *mockCryptoClient) Encrypt(ctx context.Context, in *cryptov1.EncryptRequest, opts ...grpc.CallOption) (*cryptov1.EncryptResponse, error) {
	if m.encryptFunc != nil {
		return m.encryptFunc(ctx, in, opts...)
	}
	return &cryptov1.EncryptResponse{
		Ciphertext:      []byte("mock-ciphertext"),
		KekVersionUsed:  1,
		CipherAlgorithm: "AES-256-GCM",
		CipherVersion:   1,
	}, nil
}

func (m *mockCryptoClient) Decrypt(ctx context.Context, in *cryptov1.DecryptRequest, opts ...grpc.CallOption) (*cryptov1.DecryptResponse, error) {
	if m.decryptFunc != nil {
		return m.decryptFunc(ctx, in, opts...)
	}
	return &cryptov1.DecryptResponse{
		Plaintext: []byte("mock-plaintext"),
	}, nil
}

// ── Encrypt Tests ─────────────────────────────────────────────────────────

func TestEncryptionService_Encrypt(t *testing.T) {
	mockClient := &mockCryptoClient{}
	service := NewEncryptionService(mockClient)

	resp, err := service.Encrypt(context.Background(), "BPAN123", "test_field", []byte("secret"))
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if string(resp.Ciphertext) != "mock-ciphertext" {
		t.Errorf("expected mock-ciphertext, got %s", string(resp.Ciphertext))
	}
	if resp.KekVersion != 1 {
		t.Errorf("expected kek version 1, got %d", resp.KekVersion)
	}
	if resp.CipherAlgorithm != "AES-256-GCM" {
		t.Errorf("expected AES-256-GCM, got %s", resp.CipherAlgorithm)
	}
}

func TestEncryptionService_Encrypt_RpcFailure(t *testing.T) {
	mockClient := &mockCryptoClient{
		encryptFunc: func(ctx context.Context, in *cryptov1.EncryptRequest, opts ...grpc.CallOption) (*cryptov1.EncryptResponse, error) {
			return nil, errors.New("connection refused")
		},
	}
	service := NewEncryptionService(mockClient)

	_, err := service.Encrypt(context.Background(), "BPAN123", "test_field", []byte("secret"))
	if err == nil {
		t.Fatal("expected error for RPC failure")
	}
}

func TestEncryptionService_Encrypt_FieldPassthrough(t *testing.T) {
	var capturedReq *cryptov1.EncryptRequest
	mockClient := &mockCryptoClient{
		encryptFunc: func(ctx context.Context, in *cryptov1.EncryptRequest, opts ...grpc.CallOption) (*cryptov1.EncryptResponse, error) {
			capturedReq = in
			return &cryptov1.EncryptResponse{
				Ciphertext:      []byte("ct"),
				KekVersionUsed:  2,
				CipherAlgorithm: "AES-256-GCM",
				CipherVersion:   1,
			}, nil
		},
	}
	service := NewEncryptionService(mockClient)

	_, err := service.Encrypt(context.Background(), "MY008A6FKKKLC1DH80001", "aadhaar_document", []byte("secret-doc"))
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if capturedReq.Bpan != "MY008A6FKKKLC1DH80001" {
		t.Errorf("expected BPAN passthrough, got %s", capturedReq.Bpan)
	}
	if capturedReq.FieldName != "aadhaar_document" {
		t.Errorf("expected field name passthrough, got %s", capturedReq.FieldName)
	}
}

// ── Decrypt Tests ─────────────────────────────────────────────────────────

func TestEncryptionService_Decrypt(t *testing.T) {
	mockClient := &mockCryptoClient{}
	service := NewEncryptionService(mockClient)

	plaintext, err := service.Decrypt(context.Background(), "BPAN123", "test_field", []byte("mock-ciphertext"), 1, "AES-256-GCM")
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if string(plaintext) != "mock-plaintext" {
		t.Errorf("expected mock-plaintext, got %s", string(plaintext))
	}
}

func TestEncryptionService_Decrypt_RpcFailure(t *testing.T) {
	mockClient := &mockCryptoClient{
		decryptFunc: func(ctx context.Context, in *cryptov1.DecryptRequest, opts ...grpc.CallOption) (*cryptov1.DecryptResponse, error) {
			return nil, errors.New("decryption failed")
		},
	}
	service := NewEncryptionService(mockClient)

	_, err := service.Decrypt(context.Background(), "BPAN123", "test_field", []byte("ct"), 1, "AES-256-GCM")
	if err == nil {
		t.Fatal("expected error for RPC failure")
	}
}

// ── Convenience Method Tests ──────────────────────────────────────────────

func TestEncryptionService_EncryptBatteryField(t *testing.T) {
	mockClient := &mockCryptoClient{}
	service := NewEncryptionService(mockClient)

	resp, err := service.EncryptBatteryField(context.Background(), "BPAN123", "serial_number", "SN-12345")
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if resp == nil {
		t.Fatal("expected non-nil response")
	}
}

func TestEncryptionService_DecryptBatteryField(t *testing.T) {
	mockClient := &mockCryptoClient{}
	service := NewEncryptionService(mockClient)

	plaintext, err := service.DecryptBatteryField(context.Background(), "BPAN123", "serial_number", []byte("ct"), 1, "AES-256-GCM")
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if plaintext != "mock-plaintext" {
		t.Errorf("expected mock-plaintext, got %s", plaintext)
	}
}
