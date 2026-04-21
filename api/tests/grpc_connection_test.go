package tests

import (
	"context"
	"testing"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/connectivity"
	"google.golang.org/grpc/credentials/insecure"
)

// TestGrpcConnection tests whether the Go Gateway can dial and connect
// to the backend Rust microservice.
// Note: This test requires the Rust microservice to be actively running on localhost:50051.
// If the microservice is offline, it appropriately fails after the dial timeout.
func TestGrpcConnection(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping live gRPC connection test in short mode.")
	}

	// 5-second timeout to dial the microservice
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	microserviceUrl := "127.0.0.1:50051"

	// Attempt connection using WithBlock to enforce dial-time connection
	conn, err := grpc.NewClient(
		microserviceUrl,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)

	if err != nil {
		t.Fatalf("Failed to initialize gRPC client: %v", err)
	}
	defer conn.Close()

	// Force the connection to initiate dialing
	conn.Connect()

	for {
		state := conn.GetState()
		if state == connectivity.Ready {
			break
		}
		if state == connectivity.TransientFailure || state == connectivity.Shutdown {
			t.Fatalf("Connection hit a failure state before connecting: %v", state)
		}
		
		if !conn.WaitForStateChange(ctx, state) {
			t.Fatalf("Connection timeout: failed to connect within 5 seconds! Is the Rust Core running on localhost:50051?")
		}
	}
	
	t.Logf("Successfully instantiated connection client mapped to %s! If the Rust backend is running, the services will now route correctly over port 50051.", microserviceUrl)
}
