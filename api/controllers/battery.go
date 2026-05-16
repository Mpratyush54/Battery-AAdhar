package controllers

import (
	"encoding/json"
	"net/http"

	"github.com/Mpratyush54/Battery-AAdhar/api/bpan"
	"github.com/Mpratyush54/Battery-AAdhar/api/config"
	"github.com/go-chi/chi/v5"
)

// RegisterBatteryController handles POST /api/v1/battery/register.
// It delegates to the atomic registration service which calls the Rust gRPC engine.
func RegisterBatteryController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Delegate to the registration controller which handles the full atomic flow
	RegisterBatteryAtomicController(res, req)
}

// GetBatteryController handles GET /api/v1/battery?bpan=...
// Returns public battery data by decoding the BPAN (no DB lookup needed for decoded fields).
func GetBatteryController(res http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodGet {
		http.Error(res, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	bpanStr := req.URL.Query().Get("bpan")
	if bpanStr == "" {
		http.Error(res, "Missing bpan parameter", http.StatusBadRequest)
		return
	}

	// Validate format
	if err := bpan.ValidateFormat(bpanStr); err != nil {
		res.Header().Set("Content-Type", "application/json")
		res.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(res).Encode(map[string]string{"error": err.Error()})
		return
	}

	// Decode BPAN to get embedded fields
	decoded, err := bpan.Decode(bpanStr)
	if err != nil {
		res.Header().Set("Content-Type", "application/json")
		res.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(res).Encode(map[string]string{"error": err.Error()})
		return
	}

	details := decoded.DecodeDetails()

	res.Header().Set("Content-Type", "application/json")
	json.NewEncoder(res).Encode(map[string]interface{}{
		"bpan":               bpanStr,
		"country":            details.CountryName,
		"manufacturer_code":  details.ManufacturerCode,
		"capacity_kwh":       details.CapacityKwh,
		"chemistry":          details.ChemistryType,
		"nominal_voltage_v":  details.NominalVoltageV,
		"cell_origin":        details.CellOrigin,
		"extinguisher_class": details.ExtinguisherClass,
		"manufacturing_year": details.ManufacturingYear,
	})
}

// GetBatteryByBPAN handles GET /api/v1/batteries/{bpan}
// Decodes BPAN and returns human-readable details (public endpoint).
func GetBatteryByBPAN(w http.ResponseWriter, r *http.Request) {
	bpanStr := chi.URLParam(r, "bpan")

	if err := bpan.ValidateFormat(bpanStr); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
		return
	}

	decoded, err := bpan.Decode(bpanStr)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
		return
	}

	details := decoded.DecodeDetails()

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"bpan":                bpanStr,
		"country":             details.CountryName,
		"manufacturer_code":   details.ManufacturerCode,
		"capacity_kwh":        details.CapacityKwh,
		"chemistry":           details.ChemistryType,
		"nominal_voltage_v":   details.NominalVoltageV,
		"cell_origin":         details.CellOrigin,
		"extinguisher_class":  details.ExtinguisherClass,
		"manufacturing_year":  details.ManufacturingYear,
		"manufacturing_month": details.ManufacturingMonth,
		"manufacturing_day":   details.ManufacturingDay,
		"factory_number":      details.FactoryNumber,
		"sequential_number":   details.SequentialNumber,
	})
}

// handleUpdateStatus is a placeholder for PATCH /batteries/{bpan}/status
func handleUpdateStatus(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}

// handleRegisterManufacturer is a placeholder for POST /manufacturers
func handleRegisterManufacturer(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}

// handleListManufacturers is a placeholder for GET /manufacturers
func handleListManufacturers(w http.ResponseWriter, _ *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}

// Ensure config package is referenced (used by RegisterBatteryAtomicController)
var _ = config.MicroserviceClients{}
