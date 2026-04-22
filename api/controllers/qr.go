// qr.go — QR code data access

package controllers

import (
	"net/http"
	"github.com/go-chi/chi/v5"
)

// GetQRCode — GET /api/v1/batteries/{bpan}/qr
func GetQRCode(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

// ScanQRCode — POST /api/v1/batteries/scan (upload QR payload)
func ScanQRCode(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotImplemented)
	w.Write([]byte(`{"error":"not_implemented"}`))
}

func RegisterQRRoutes(r chi.Router) {
	r.Get("/batteries/{bpan}/qr", GetQRCode)
	r.Post("/batteries/scan", ScanQRCode)
}
