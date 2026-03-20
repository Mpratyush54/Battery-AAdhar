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
	
	httpSwagger "github.com/swaggo/http-swagger"
	_ "api/docs"

	pb "api/pb"
)

const microserviceUrl = "127.0.0.1:50051"

// RidePayload represents the incoming request to create a ride
// @Description Payload containing the zero-knowledge proof and ride details
type RidePayload struct {
	ZkProof     string `json:"zkProof" example:"proof_payload_123"`
	RideDetails string `json:"rideDetails" example:"New ride details..."`
}

// RideResponseJSON represents the ride object returned by the system
// @Description Complete ride details including its assigned ID
type RideResponseJSON struct {
	ID          int32  `json:"id" example:"42"`
	ZkProof     string `json:"zkProof" example:"proof_payload_123"`
	RideDetails string `json:"rideDetails" example:"New ride details..."`
}

type RegisterBatteryPayload struct {
	ManufacturerID    string  `json:"manufacturerId" example:"123e4567-e89b-12d3-a456-426614174000"`
	ManufacturerCode  string  `json:"manufacturerCode" example:"ABC"`
	ChemistryType     string  `json:"chemistryType" example:"LFP"`
	BatteryCategory   string  `json:"batteryCategory" example:"EV-M"`
	ComplianceClass   string  `json:"complianceClass" example:"AIS-156"`
	NominalVoltage    float64 `json:"nominalVoltage" example:"48.0"`
	RatedCapacityKwh  float64 `json:"ratedCapacityKwh" example:"2.5"`
	EnergyDensity     float64 `json:"energyDensity" example:"150.0"`
	WeightKg          float64 `json:"weightKg" example:"25.0"`
	FormFactor        string  `json:"formFactor" example:"PRISMATIC"`
	SerialNumber      string  `json:"serialNumber" example:"SN12345678"`
	BatchNumber       string  `json:"batchNumber" example:"BAT2026"`
	FactoryCode       string  `json:"factoryCode" example:"FAC01"`
	ProductionYear    uint32  `json:"productionYear" example:"2026"`
	SequenceNumber    string  `json:"sequenceNumber" example:"01"`
	ActorID           string  `json:"actorId" example:"123e4567-e89b-12d3-a456-426614174000"`
}

type RegisterBatteryResponseJSON struct {
	Bpan           string `json:"bpan"`
	StaticHash     string `json:"staticHash"`
	RegistrationId string `json:"registrationId"`
	Status         string `json:"status"`
}

var rideService pb.RideServiceClient
var bpaService pb.BpaServiceClient

func initMicroserviceClient() {
	connection, err := grpc.NewClient(microserviceUrl, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatalf("Microservice connection failed: %v", err)
	}
	rideService = pb.NewRideServiceClient(connection)
	bpaService = pb.NewBpaServiceClient(connection)
}

// createRideController godoc
// @Summary Create a new ride
// @Description Creates a ride by passing a ZkProof and details to the Rust gRPC backend
// @Tags rides
// @Accept json
// @Produce json
// @Param payload body RidePayload true "Ride creation payload"
// @Success 200 {object} RideResponseJSON "Successful ride creation"
// @Failure 400 {string} string "Invalid payload"
// @Failure 405 {string} string "Method not allowed"
// @Failure 500 {string} string "Internal Server/Microservice Error"
// @Router /api/v1/rides [post]
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

// getRidesController godoc
// @Summary Get all rides
// @Description Retrieves all rides decrypted by the Rust backend
// @Tags rides
// @Produce json
// @Success 200 {array} RideResponseJSON "List of rides"
// @Failure 405 {string} string "Method not allowed"
// @Failure 500 {string} string "Internal Server/Microservice Error"
// @Router /api/v1/rides [get]
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

// registerBatteryController godoc
// @Summary Register a new battery
// @Description Registers a new battery with the BPA Core Engine
// @Tags battery
// @Accept json
// @Produce json
// @Param payload body RegisterBatteryPayload true "Battery registration payload"
// @Success 200 {object} RegisterBatteryResponseJSON "Successful registration"
// @Failure 400 {string} string "Invalid payload"
// @Failure 405 {string} string "Method not allowed"
// @Failure 500 {string} string "Internal Server/Microservice Error"
// @Router /api/v1/battery/register [post]
func registerBatteryController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	bodyBuffer, err := io.ReadAll(req.Body)
	if err != nil {
		http.Error(res, "Error reading request body", http.StatusInternalServerError)
		return
	}

	var payload RegisterBatteryPayload
	if err := json.Unmarshal(bodyBuffer, &payload); err != nil {
		http.Error(res, "Invalid payload", http.StatusBadRequest)
		return
	}

	ctx, cancel := context.WithTimeout(context.Background(), time.Second*5)
	defer cancel()

	response, err := bpaService.RegisterBattery(ctx, &pb.RegisterBatteryRequest{
		ManufacturerId:    payload.ManufacturerID,
		ManufacturerCode:  payload.ManufacturerCode,
		ChemistryType:     payload.ChemistryType,
		BatteryCategory:   payload.BatteryCategory,
		ComplianceClass:   payload.ComplianceClass,
		NominalVoltage:    payload.NominalVoltage,
		RatedCapacityKwh:  payload.RatedCapacityKwh,
		EnergyDensity:     payload.EnergyDensity,
		WeightKg:          payload.WeightKg,
		FormFactor:        payload.FormFactor,
		SerialNumber:      payload.SerialNumber,
		BatchNumber:       payload.BatchNumber,
		FactoryCode:       payload.FactoryCode,
		ProductionYear:    payload.ProductionYear,
		SequenceNumber:    payload.SequenceNumber,
		ActorId:           payload.ActorID,
	})

	if err != nil {
		log.Printf("Microservice error: %v", err)
		http.Error(res, "Microservice error: "+err.Error(), http.StatusInternalServerError)
		return
	}

	jsonResponse := RegisterBatteryResponseJSON{
		Bpan:           response.GetBpan(),
		StaticHash:     response.GetStaticHash(),
		RegistrationId: response.GetRegistrationId(),
		Status:         response.GetStatus(),
	}

	res.Header().Set("Content-Type", "application/json")
	json.NewEncoder(res).Encode(jsonResponse)
}

// @title Battery Pack Aadhaar / zk_rides API
// @version 1.0
// @description The Go Gateway for the BPA and Zero-Knowledge Rides application.
// @host localhost:3000
// @BasePath /
func main() {
	initMicroserviceClient()

	expressRouter := http.NewServeMux()

	expressRouter.HandleFunc("/api/v1/rides", func(res http.ResponseWriter, req *http.Request) {
		switch req.Method {
		case http.MethodPost:
			createRideController(res, req)
		case http.MethodGet:
			getRidesController(res, req)
		default:
			http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		}
	})

	expressRouter.HandleFunc("/api/v1/battery/register", func(res http.ResponseWriter, req *http.Request) {
		switch req.Method {
		case http.MethodPost:
			registerBatteryController(res, req)
		default:
			http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		}
	})

	// Add Swagger HTTP handler
	expressRouter.HandleFunc("/swagger/", httpSwagger.WrapHandler)

	log.Println("API Gateway running on port 3000 (Proxying to Microservice)")
	log.Println("Swagger documentation available at http://localhost:3000/swagger/index.html")
	if err := http.ListenAndServe(":3000", expressRouter); err != nil {
		log.Fatalf("Could not start Express-like server: %v", err)
	}
}
