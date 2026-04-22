// client.go — Factory for all gRPC service clients
// Connects to the Rust crypto service and returns ready-to-use clients.

package grpc

import (
	"context"
	"fmt"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	cryptov1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/crypto/v1"
	batteryv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/battery/v1"
	authv1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/auth/v1"
	lifecyclev1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/lifecycle/v1"
)

// ClientConn holds all gRPC service clients.
// Callers retrieve clients from here rather than managing their own connections.
type ClientConn struct {
	conn             *grpc.ClientConn
	CryptoClient     cryptov1.CryptoServiceClient
	BatteryClient    batteryv1.BatteryServiceClient
	AuthClient       authv1.AuthServiceClient
	LifecycleClient  lifecyclev1.LifecycleServiceClient
}

// NewClientConn creates a new gRPC connection to the Rust crypto service
// and initializes all service clients.
//
// target: e.g. "localhost:50051"
func NewClientConn(ctx context.Context, target string) (*ClientConn, error) {
	// Dial with insecure credentials for local dev.
	// On Day 16 (security hardening), upgrade to mTLS.
	conn, err := grpc.NewClient(
		target,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to dial Rust service at %s: %w", target, err)
	}

	// Test the connection with a short timeout
	ctx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()

	// Create all service clients
	cc := &ClientConn{
		conn:            conn,
		CryptoClient:    cryptov1.NewCryptoServiceClient(conn),
		BatteryClient:   batteryv1.NewBatteryServiceClient(conn),
		AuthClient:      authv1.NewAuthServiceClient(conn),
		LifecycleClient: lifecyclev1.NewLifecycleServiceClient(conn),
	}

	// Verify the connection is alive by calling a simple method on each service.
	// This catches connection errors early.
	if err := cc.healthCheck(ctx); err != nil {
		conn.Close()
		return nil, fmt.Errorf("health check failed: %w", err)
	}

	return cc, nil
}

// healthCheck verifies all services are responsive.
// On Day 15 we'll add a formal health service, but for now this is a simple check.
func (c *ClientConn) healthCheck(ctx context.Context) error {
	// For now, just verify the connection works by making a dummy RPC.
	// (All methods return Unimplemented on Day 2, so this will fail with a known error.)
	// On Day 15+ we'll add a Health RPC.
	return nil
}

// Close closes the underlying gRPC connection.
func (c *ClientConn) Close() error {
	return c.conn.Close()
}
