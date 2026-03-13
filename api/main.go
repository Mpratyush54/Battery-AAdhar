package main

import (
	"context"
	"encoding/json"
	"io"
	"log"
	"net/http"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	pb "api/pb"
)

const microserviceUrl = "localhost:50051"

type RidePayload struct {
	ZkProof     string `json:"zkProof"`
	RideDetails string `json:"rideDetails"`
}

type RideResponseJSON struct {
	ID          int32  `json:"id"`
	ZkProof     string `json:"zkProof"`
	RideDetails string `json:"rideDetails"`
}

var rideService pb.RideServiceClient

func initMicroserviceClient() {
	connection, err := grpc.NewClient(microserviceUrl, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatalf("Microservice connection failed: %v", err)
	}
	rideService = pb.NewRideServiceClient(connection)
}

func createRideController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	bodyBuffer, err := io.ReadAll(req.Body)
	if err != nil {
		http.Error(res, "Error reading request body", http.StatusInternalServerError)
		return
	}

	var payload RidePayload
	if err := json.Unmarshal(bodyBuffer, &payload); err != nil {
		http.Error(res, "Invalid payload. Needs zkProof and rideDetails", http.StatusBadRequest)
		return
	}

	// Await promise-like context
	ctx, cancel := context.WithTimeout(context.Background(), time.Second*5)
	defer cancel()

	response, err := rideService.CreateRide(ctx, &pb.CreateRideRequest{
		ZkProof:     payload.ZkProof,
		RideDetails: payload.RideDetails,
	})
	if err != nil {
		log.Printf("Microservice error: %v", err)
		http.Error(res, "Microservice error: "+err.Error(), http.StatusInternalServerError)
		return
	}

	jsonResponse := RideResponseJSON{
		ID:          response.GetId(),
		ZkProof:     response.GetZkProof(),
		RideDetails: response.GetRideDetails(),
	}

	res.Header().Set("Content-Type", "application/json")
	json.NewEncoder(res).Encode(jsonResponse)
}

func getRidesController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodGet {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	ctx, cancel := context.WithTimeout(context.Background(), time.Second*5)
	defer cancel()

	response, err := rideService.GetRides(ctx, &pb.GetRidesRequest{})
	if err != nil {
		log.Printf("Microservice error: %v", err)
		http.Error(res, "Microservice error: "+err.Error(), http.StatusInternalServerError)
		return
	}

	var responseArray []RideResponseJSON
	for _, item := range response.GetRides() {
		responseArray = append(responseArray, RideResponseJSON{
			ID:          item.GetId(),
			ZkProof:     item.GetZkProof(),
			RideDetails: item.GetRideDetails(),
		})
	}

	if responseArray == nil {
		responseArray = []RideResponseJSON{}
	}

	res.Header().Set("Content-Type", "application/json")
	json.NewEncoder(res).Encode(responseArray)
}

func main() {
	initMicroserviceClient()

	expressRouter := http.NewServeMux()

	expressRouter.HandleFunc("/api/v1/rides", func(res http.ResponseWriter, req *http.Request) {
		if req.Method == http.MethodPost {
			createRideController(res, req)
		} else if req.Method == http.MethodGet {
			getRidesController(res, req)
		} else {
			http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		}
	})

	log.Println("API Gateway running on port 3000 (Proxying to Microservice)")
	if err := http.ListenAndServe(":3000", expressRouter); err != nil {
		log.Fatalf("Could not start Express-like server: %v", err)
	}
}
