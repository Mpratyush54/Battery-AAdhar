// microservice.go — Microservice configuration and client initialization

package config

import (
	"context"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
)

// MicroserviceClients holds all initialized gRPC clients.
// Attach this to your app context/state so handlers can use it.
type MicroserviceClients struct {
	GrpcConn *grpc.ClientConn
}

// InitMicroservices initializes all gRPC service clients.
func InitMicroservices(ctx context.Context, cryptoServiceTarget string) (*MicroserviceClients, error) {
	// Connect to the Rust crypto service
	cc, err := grpc.NewClientConn(ctx, cryptoServiceTarget)
	if err != nil {
		return nil, err
	}

	return &MicroserviceClients{
		GrpcConn: cc,
	}, nil
}

// Close closes all connections.
func (m *MicroserviceClients) Close() error {
	return m.GrpcConn.Close()
}
