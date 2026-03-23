package config

import (
	pb "api/pb"
	"context"
	"log"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/connectivity"
	"google.golang.org/grpc/credentials/insecure"
)

const microserviceUrl = "127.0.0.1:50051"
var BpaService pb.BpaServiceClient
var AuthService pb.AuthServiceClient

func InitMicroserviceClient() {
	connection, err := grpc.NewClient(microserviceUrl, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatalf("Microservice setup failed: %v", err)
	}

	connection.Connect() // Trigger eager connection
	log.Printf("⏳ Waiting for gRPC microservice connection at %s...", microserviceUrl)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	for {
		state := connection.GetState()
		if state == connectivity.Ready {
			log.Println("✅ Successfully connected to Rust gRPC Microservice!")
			break
		}
		if !connection.WaitForStateChange(ctx, state) {
			log.Fatalf("❌ CRITICAL: Failed to connect to Rust gRPC microservice at %s within 5 seconds. Please ensure the Rust backend is running!", microserviceUrl)
		}
	}

	BpaService = pb.NewBpaServiceClient(connection)
	AuthService = pb.NewAuthServiceClient(connection)
}
