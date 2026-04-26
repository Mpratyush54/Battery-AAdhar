// client.go — Factory for all gRPC service clients
// Connects to the Rust crypto service and returns ready-to-use clients.
// Supports both mTLS (production) and insecure (local dev) modes.

package grpc

import (
	"context"
	"crypto/tls"
	"crypto/x509"
	"fmt"
	"log/slog"
	"os"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
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
// If GRPC_CA_CERT, GRPC_CLIENT_CERT, GRPC_CLIENT_KEY are all set, uses mTLS.
// Otherwise falls back to insecure transport for local development.
//
// target: e.g. "localhost:50051"
func NewClientConn(ctx context.Context, target string) (*ClientConn, error) {
	var dialOpt grpc.DialOption

	caCertFile := os.Getenv("GRPC_CA_CERT")
	clientCertFile := os.Getenv("GRPC_CLIENT_CERT")
	clientKeyFile := os.Getenv("GRPC_CLIENT_KEY")

	if caCertFile != "" && clientCertFile != "" && clientKeyFile != "" {
		// mTLS mode — load certificates
		cert, err := tls.LoadX509KeyPair(clientCertFile, clientKeyFile)
		if err != nil {
			return nil, fmt.Errorf("failed to load client certificates: %w", err)
		}

		caCert, err := os.ReadFile(caCertFile)
		if err != nil {
			return nil, fmt.Errorf("failed to read CA certificate: %w", err)
		}
		caCertPool := x509.NewCertPool()
		if !caCertPool.AppendCertsFromPEM(caCert) {
			return nil, fmt.Errorf("failed to append CA certificate")
		}

		tlsConfig := &tls.Config{
			Certificates: []tls.Certificate{cert},
			RootCAs:      caCertPool,
			ServerName:   "localhost",
		}
		dialOpt = grpc.WithTransportCredentials(credentials.NewTLS(tlsConfig))
		slog.Info("gRPC client using mTLS", "target", target)
	} else {
		// Insecure mode — local dev without TLS on Rust side
		dialOpt = grpc.WithTransportCredentials(insecure.NewCredentials())
		slog.Debug("gRPC client using INSECURE transport (dev only)", "target", target)
	}

	conn, err := grpc.NewClient(target, dialOpt)
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

// healthCheck performs a real handshake with the Rust engine.
// Calls ZkProve(proof_type=OPERATIONAL, value=85) — a lightweight in-memory
// ZK proof that requires no DB and confirms the crypto engine is live.
func (c *ClientConn) healthCheck(ctx context.Context) error {
	slog.Info("🤝 Performing handshake with Rust gRPC engine...")

	resp, err := c.CryptoClient.ZkProve(ctx, &cryptov1.ZkProveRequest{
		ProofType: 1,   // OPERATIONAL
		Value:     85,  // SoH 85% — safely within operational threshold (>80%)
		RangeMin:  80,
		RangeMax:  100,
	})
	if err != nil {
		return fmt.Errorf("ZkProve handshake failed: %w", err)
	}

	slog.Info("✅ Rust engine handshake OK",
		"proof_bytes", len(resp.Proof),
		"commitment_bytes", len(resp.PublicInputs),
	)
	return nil
}

// Close closes the underlying gRPC connection.
func (c *ClientConn) Close() error {
	return c.conn.Close()
}
