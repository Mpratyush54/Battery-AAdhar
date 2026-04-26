package tests

import (
	"context"
	"testing"
	"time"

	bpa_grpc "github.com/Mpratyush54/Battery-AAdhar/api/grpc"
)

// TestGrpcConnection tests whether the Go Gateway can dial and connect
// to the backend Rust microservice.
//
// For local dev (no Docker, no TLS on Rust side), this uses insecure transport.
// In production (mTLS certs configured), it will use mutual TLS.
//
// Requires: Rust core running on localhost:50051 (cargo run from core/).
func TestGrpcConnection(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping live gRPC connection test in short mode.")
	}

	// 5-second timeout to dial the microservice
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	microserviceUrl := "localhost:50051"

	// NewClientConn auto-detects mTLS vs insecure based on env vars
	clientConn, err := bpa_grpc.NewClientConn(ctx, microserviceUrl)
	if err != nil {
		t.Fatalf("Failed to initialize gRPC client: %v", err)
	}
	defer clientConn.Close()

	t.Logf("Successfully connected to Rust core at %s", microserviceUrl)
}
