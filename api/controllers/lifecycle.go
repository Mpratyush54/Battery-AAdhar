// lifecycle.go — Battery lifecycle transitions

package controllers

import (
	"net/http"
	"github.com/go-chi/chi/v5"
)

// TransferOwnership — POST /api/v1/batteries/{bpan}/ownership/transfer
func TransferOwnership(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// GetOwnershipHistory — GET /api/v1/batteries/{bpan}/ownership/history
func GetOwnershipHistory(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// CertifyReuse — POST /api/v1/batteries/{bpan}/reuse (service provider)
func CertifyReuse(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// RecordRecycling — POST /api/v1/batteries/{bpan}/recycling (recycler)
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
