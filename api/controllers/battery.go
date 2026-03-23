package controllers

import (
	"context"
	"encoding/json"
	"io"
	"log"
	"net/http"
	"time"

	"api/config"
	"api/models"
	pb "api/pb"
)

// RegisterBatteryController godoc
// @Summary Register a new battery
// @Description Registers a new battery with the BPA Core Engine
// @Tags battery
// @Accept json
// @Produce json
// @Param payload body models.RegisterBatteryPayload true "Battery registration payload"
// @Success 200 {object} models.RegisterBatteryResponseJSON "Successful registration"
// @Failure 400 {string} string "Invalid payload"
// @Failure 405 {string} string "Method not allowed"
// @Failure 500 {string} string "Internal Server/Microservice Error"
// @Router /api/v1/battery/register [post]
func RegisterBatteryController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	bodyBuffer, err := io.ReadAll(req.Body)
	if err != nil {
		http.Error(res, "Error reading request body", http.StatusInternalServerError)
		return
	}

	var payload models.RegisterBatteryPayload
	if err := json.Unmarshal(bodyBuffer, &payload); err != nil {
		http.Error(res, "Invalid payload", http.StatusBadRequest)
		return
	}

	ctx, cancel := context.WithTimeout(context.Background(), time.Second*5)
	defer cancel()

	if config.BpaService == nil {
		http.Error(res, "BPA Service unavailable", http.StatusInternalServerError)
		return
	}

	response, err := config.BpaService.RegisterBattery(ctx, &pb.RegisterBatteryRequest{
		ManufacturerId:   payload.ManufacturerID,
		ManufacturerCode: payload.ManufacturerCode,
		ChemistryType:    payload.ChemistryType,
		BatteryCategory:  payload.BatteryCategory,
		ComplianceClass:  payload.ComplianceClass,
		NominalVoltage:   payload.NominalVoltage,
		RatedCapacityKwh: payload.RatedCapacityKwh,
		EnergyDensity:    payload.EnergyDensity,
		WeightKg:         payload.WeightKg,
		FormFactor:       payload.FormFactor,
		SerialNumber:     payload.SerialNumber,
		BatchNumber:      payload.BatchNumber,
		FactoryCode:      payload.FactoryCode,
		ProductionYear:   payload.ProductionYear,
		SequenceNumber:   payload.SequenceNumber,
		ActorId:          payload.ActorID,
	})

	if err != nil {
		log.Printf("Microservice error: %v", err)
		http.Error(res, "Microservice error: "+err.Error(), http.StatusInternalServerError)
		return
	}

	jsonResponse := models.RegisterBatteryResponseJSON{
		Bpan:           response.GetBpan(),
		StaticHash:     response.GetStaticHash(),
		RegistrationId: response.GetRegistrationId(),
		Status:         response.GetStatus(),
	}

	res.Header().Set("Content-Type", "application/json")
	json.NewEncoder(res).Encode(jsonResponse)
}
