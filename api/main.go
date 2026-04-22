// main.go — Battery Aadhaar API server
// HTTP router + gRPC client factory

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
	// Initialize gRPC clients
	grpcTarget := os.Getenv("GRPC_SERVICE_URL")
	if grpcTarget == "" {
		grpcTarget = "localhost:50051"
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	microservices, err := config.InitMicroservices(ctx, grpcTarget)
	if err != nil {
		log.Fatalf("Failed to initialize gRPC clients: %v", err)
	}
	defer microservices.Close()

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
		log.Fatalf("Server error: %v", err)
	}
}
