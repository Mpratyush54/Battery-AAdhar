// main.go — Battery Aadhaar API server
// HTTP router + gRPC client factory

// @title           Battery Pack Aadhaar (BPA) API
// @version         2.0
// @description     Zero-Knowledge Battery Authentication Platform — REST API Gateway
// @termsOfService  https://bpa.pratyushes.dev/terms

// @contact.name   BPA Engineering Team
// @contact.email  bpa-dev@bpa.pratyushes.dev

// @license.name  Apache 2.0
// @license.url   http://www.apache.org/licenses/LICENSE-2.0.html

// @host      localhost:8080
// @BasePath  /api/v1

// @securityDefinitions.apikey BearerAuth
// @in header
// @name Authorization
// @description JWT Bearer token (access_token cookie or Authorization header)

package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/Mpratyush54/Battery-AAdhar/api/config"
	"github.com/Mpratyush54/Battery-AAdhar/api/routes"
)

func main() {
	// Initialize DB
	config.InitDB()

	// Initialize gRPC clients — non-fatal if Rust server is not yet running
	grpcTarget := os.Getenv("GRPC_SERVICE_URL")
	if grpcTarget == "" {
		grpcTarget = "localhost:50051"
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	microservices, err := config.InitMicroservices(ctx, grpcTarget)
	if err != nil {
		log.Printf("⚠️  gRPC connection to Rust engine failed: %v", err)
		log.Println("⚠️  Continuing without Rust engine — crypto/ZK endpoints will return 503")
		microservices = nil
	} else {
		log.Println("✅ Connected to Rust gRPC engine")
		defer microservices.Close()
	}

	// Create router
	router := routes.NewRouter()

	// Start HTTP server
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	addr := fmt.Sprintf(":%s", port)
	log.Printf("🚀 BPA API server starting on %s", addr)

	if err := http.ListenAndServe(addr, router); err != nil {
		log.Fatalf("❌ Server error: %v", err)
	}
}
