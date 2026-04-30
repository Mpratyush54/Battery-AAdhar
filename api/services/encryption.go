// encryption.go — Encryption service proxy
// All encryption/decryption is delegated to the Rust gRPC service.
// Go layer never handles plaintext sensitive data.

package services

import (
	"context"
	"fmt"

	cryptov1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/crypto/v1"
)

// EncryptionService proxies all encrypt/decrypt operations to Rust
type EncryptionService struct {
	cryptoClient cryptov1.CryptoServiceClient
}

// NewEncryptionService creates a new encryption proxy
func NewEncryptionService(cryptoClient cryptov1.CryptoServiceClient) *EncryptionService {
	return &EncryptionService{
		cryptoClient: cryptoClient,
	}
}

// EncryptedData represents the output of an encryption operation
type EncryptedData struct {
	Ciphertext      []byte
	KekVersion      int32
	CipherAlgorithm string
	CipherVersion   int32
}

// Encrypt sends a plaintext field to Rust for encryption via AES-256-GCM
// The Rust service encrypts and returns the ciphertext + metadata.
func (e *EncryptionService) Encrypt(
	ctx context.Context,
	bpan string,
	fieldName string,
	plaintext []byte,
) (*EncryptedData, error) {
	req := &cryptov1.EncryptRequest{
		Bpan:       bpan,
		FieldName:  fieldName,
		Plaintext:  plaintext,
		KekVersion: 0, // Use current version
	}

	resp, err := e.cryptoClient.Encrypt(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("encrypt RPC failed: %w", err)
	}

	return &EncryptedData{
		Ciphertext:      resp.Ciphertext,
		KekVersion:      resp.KekVersionUsed,
		CipherAlgorithm: resp.CipherAlgorithm,
		CipherVersion:   resp.CipherVersion,
	}, nil
}

// Decrypt sends a ciphertext to Rust for decryption
// Returns the plaintext.
func (e *EncryptionService) Decrypt(
	ctx context.Context,
	bpan string,
	fieldName string,
	ciphertext []byte,
	kekVersion int32,
	cipherAlgorithm string,
) ([]byte, error) {
	req := &cryptov1.DecryptRequest{
		Bpan:            bpan,
		FieldName:       fieldName,
		Ciphertext:      ciphertext,
		KekVersion:      kekVersion,
		CipherAlgorithm: cipherAlgorithm,
	}

	resp, err := e.cryptoClient.Decrypt(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("decrypt RPC failed: %w", err)
	}

	return resp.Plaintext, nil
}

// EncryptBatteryField is a convenience method for encrypting a single battery field
func (e *EncryptionService) EncryptBatteryField(
	ctx context.Context,
	bpan string,
	fieldName string,
	value string,
) (*EncryptedData, error) {
	return e.Encrypt(ctx, bpan, fieldName, []byte(value))
}

// DecryptBatteryField is a convenience method for decrypting a single battery field
func (e *EncryptionService) DecryptBatteryField(
	ctx context.Context,
	bpan string,
	fieldName string,
	ciphertext []byte,
	kekVersion int32,
	cipherAlgorithm string,
) (string, error) {
	plaintext, err := e.Decrypt(ctx, bpan, fieldName, ciphertext, kekVersion, cipherAlgorithm)
	if err != nil {
		return "", err
	}
	return string(plaintext), nil
}
