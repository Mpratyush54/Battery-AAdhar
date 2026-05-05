// grpc.go — gRPC microservice client initialisation
//
// Connects to the Rust BPA engine at GRPC_TARGET (default 127.0.0.1:50051).
// TLS priority:
//   1. GRPC_CA_CERT_PEM / GRPC_CLIENT_CERT_PEM / GRPC_CLIENT_KEY_PEM env vars
//   2. Same secrets fetched from Infisical
//   3. Insecure fallback (dev only)

package config

import (
	"context"
	cryptotls "crypto/tls"
	"crypto/x509"
	"log"
	"os"
	"time"

	pb "github.com/Mpratyush54/Battery-AAdhar/api/pb"
	infisical "github.com/infisical/go-sdk"
	"google.golang.org/grpc"
	"google.golang.org/grpc/connectivity"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/credentials/insecure"
)

var BpaService pb.BpaServiceClient

// grpcTLSPEM holds resolved PEM bytes for mTLS.
type grpcTLSPEM struct {
	caCertPEM     []byte
	clientCertPEM []byte
	clientKeyPEM  []byte
}

// loadGrpcTLS resolves TLS PEM contents from env or Infisical.
// Returns nil → insecure transport.
func loadGrpcTLS() *grpcTLSPEM {
	caPEM := os.Getenv("GRPC_CA_CERT_PEM")
	certPEM := os.Getenv("GRPC_CLIENT_CERT_PEM")
	keyPEM := os.Getenv("GRPC_CLIENT_KEY_PEM")

	if caPEM != "" && certPEM != "" && keyPEM != "" {
		log.Println("✅ gRPC TLS credentials loaded from environment")
		return &grpcTLSPEM{[]byte(caPEM), []byte(certPEM), []byte(keyPEM)}
	}

	// Try Infisical
	clientID := os.Getenv("INFISICAL_CLIENT_ID")
	clientSecret := os.Getenv("INFISICAL_CLIENT_SECRET")
	projectID := os.Getenv("INFISICAL_PROJECT_ID")
	env := os.Getenv("INFISICAL_ENV")
	if env == "" {
		env = "dev"
	}

	if clientID == "" || clientSecret == "" {
		return nil
	}

	log.Println("🔐 Fetching gRPC TLS credentials from Infisical...")
	ic := infisical.NewInfisicalClient(context.Background(), infisical.Config{
		SiteUrl: os.Getenv("INFISICAL_BASE_URL"),
	})
	if _, err := ic.Auth().UniversalAuthLogin(clientID, clientSecret); err != nil {
		log.Printf("❌ Infisical auth failed (gRPC TLS): %v", err)
		return nil
	}

	fetch := func(key string) string {
		s, err := ic.Secrets().Retrieve(infisical.RetrieveSecretOptions{
			SecretKey:   key,
			Environment: env,
			ProjectID:   projectID,
			SecretPath:  "/",
		})
		if err != nil {
			log.Printf("⚠️  %s not found in Infisical: %v", key, err)
			return ""
		}
		log.Printf("✅ %s retrieved from Infisical", key)
		return s.SecretValue
	}

	if caPEM == "" {
		caPEM = fetch("GRPC_CA_CERT_PEM")
	}
	if certPEM == "" {
		certPEM = fetch("GRPC_CLIENT_CERT_PEM")
	}
	if keyPEM == "" {
		keyPEM = fetch("GRPC_CLIENT_KEY_PEM")
	}

	if caPEM == "" || certPEM == "" || keyPEM == "" {
		log.Println("⚠️  One or more gRPC TLS secrets missing — falling back to insecure")
		return nil
	}

	return &grpcTLSPEM{[]byte(caPEM), []byte(certPEM), []byte(keyPEM)}
}

// buildDialOpt returns the grpc.DialOption for the connection.
func buildDialOpt(target string, pem *grpcTLSPEM) grpc.DialOption {
	if pem == nil {
		log.Printf("⚠️  gRPC using INSECURE transport (dev only) — target=%s", target)
		return grpc.WithTransportCredentials(insecure.NewCredentials())
	}

	cert, err := cryptotls.X509KeyPair(pem.clientCertPEM, pem.clientKeyPEM)
	if err != nil {
		log.Printf("❌ Failed to parse client cert/key: %v — falling back to insecure", err)
		return grpc.WithTransportCredentials(insecure.NewCredentials())
	}

	caPool := x509.NewCertPool()
	if !caPool.AppendCertsFromPEM(pem.caCertPEM) {
		log.Println("❌ Failed to parse CA cert PEM — falling back to insecure")
		return grpc.WithTransportCredentials(insecure.NewCredentials())
	}

	tlsCfg := &cryptotls.Config{
		Certificates: []cryptotls.Certificate{cert},
		RootCAs:      caPool,
		ServerName:   "localhost",
	}
	log.Printf("🔒 gRPC client using mTLS — target=%s", target)
	return grpc.WithTransportCredentials(credentials.NewTLS(tlsCfg))
}

// InitMicroserviceClient connects to the Rust BPA engine.
// Target is read from GRPC_TARGET env var (default 127.0.0.1:50051).
func InitMicroserviceClient() {
	target := os.Getenv("GRPC_TARGET")
	if target == "" {
		target = "127.0.0.1:50051"
	}

	dialOpt := buildDialOpt(target, loadGrpcTLS())

	connection, err := grpc.NewClient(target, dialOpt)
	if err != nil {
		log.Fatalf("Microservice setup failed: %v", err)
	}

	connection.Connect()
	log.Printf("⏳ Waiting for gRPC microservice connection at %s...", target)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	for {
		state := connection.GetState()
		if state == connectivity.Ready {
			log.Println("✅ Successfully connected to Rust gRPC Microservice!")
			break
		}
		if !connection.WaitForStateChange(ctx, state) {
			log.Printf("⚠️  Failed to connect to %s within 5s — running in decoupled mode", target)
			break
		}
	}

	BpaService = pb.NewBpaServiceClient(connection)
}
