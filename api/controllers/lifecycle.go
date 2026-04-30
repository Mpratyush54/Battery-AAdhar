// lifecycle.go — Battery lifecycle transitions

package controllers

import (
	"github.com/go-chi/chi/v5"
	"net/http"
)

// TransferOwnership godoc
// @Summary      Transfer battery ownership
// @Description  Records an ownership transfer event for a battery
// @Tags         lifecycle
// @Accept       json
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Param        body   body   object  true  "Ownership transfer payload"
// @Success      200  {object}  map[string]interface{}
// @Failure      400  {object}  map[string]string
// @Failure      403  {object}  map[string]string  "Forbidden"
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/ownership/transfer [post]
// @Security     BearerAuth
func TransferOwnership(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetOwnershipHistory godoc
// @Summary      Get ownership history
// @Description  Returns the full chain of ownership for a battery
// @Tags         lifecycle
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Success      200  {array}   map[string]interface{}
// @Failure      404  {object}  map[string]string
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/ownership/history [get]
// @Security     BearerAuth
func GetOwnershipHistory(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// CertifyReuse godoc
// @Summary      Certify battery for second-life reuse
// @Description  Records a reuse certification event (reuse operator)
// @Tags         lifecycle
// @Accept       json
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Param        body   body   object  true  "Reuse certification payload"
// @Success      200  {object}  map[string]interface{}
// @Failure      403  {object}  map[string]string  "Forbidden"
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/reuse [post]
// @Security     BearerAuth
func CertifyReuse(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// RecordRecycling godoc
// @Summary      Record battery recycling
// @Description  Records a recycling event with material recovery data (recycler only)
// @Tags         lifecycle
// @Accept       json
// @Produce      json
// @Param        bpan   path   string  true  "Battery PAN"
// @Param        body   body   object  true  "Recycling record payload"
// @Success      200  {object}  map[string]interface{}
// @Failure      403  {object}  map[string]string  "Forbidden"
// @Failure      501  {object}  map[string]string  "Not implemented"
// @Router       /batteries/{bpan}/recycling [post]
// @Security     BearerAuth
func RecordRecycling(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

func RegisterLifecycleRoutes(r chi.Router) {
	r.Post("/batteries/{bpan}/ownership/transfer", TransferOwnership)
	r.Get("/batteries/{bpan}/ownership/history", GetOwnershipHistory)
	r.Post("/batteries/{bpan}/reuse", CertifyReuse)
	r.Post("/batteries/{bpan}/recycling", RecordRecycling)
}
