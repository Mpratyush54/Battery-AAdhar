// lifecycle.go — Battery lifecycle HTTP handlers (Day 12)
// Wraps the LifecycleService gRPC bridge for state transitions and ownership transfers.

package controllers

import (
	"encoding/json"
	"net/http"

	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
	"github.com/Mpratyush54/Battery-AAdhar/api/services"
	"github.com/go-chi/chi/v5"
)

// RegisterLifecycleRoutes wires all lifecycle endpoints onto the given router.
func RegisterLifecycleRoutes(r chi.Router, s *services.LifecycleService) {
	r.Group(func(r chi.Router) {
		r.Use(middleware.Authenticate)

		// Ownership transfers (any authenticated party)
		r.Post("/batteries/{bpan}/ownership/transfer", TransferOwnership(s))
		r.Post("/ownership/transfer/{id}/confirm", ConfirmTransfer(s))
		r.Post("/ownership/transfer/{id}/reject", RejectTransfer(s))
		r.Get("/batteries/{bpan}/ownership/history", GetOwnershipHistory(s))

		// Lifecycle state transitions (manufacturer / operator)
		r.Post("/batteries/{bpan}/transition", TransitionState(s))
		r.Post("/batteries/{bpan}/reuse", CertifyReuse(s))
		r.Post("/batteries/{bpan}/recycling", RecordRecycling(s))

		// Verification (verifier role only)
		r.Group(func(r chi.Router) {
			r.Use(middleware.IsRole("verifier"))
			r.Post("/batteries/{bpan}/verify/operational", VerifyOperational(s))
			r.Post("/batteries/{bpan}/verify/signature", VerifySignature(s))
		})
	})
}

// ── TransitionState ────────────────────────────────────────────────────────────

// TransitionState godoc
// @Summary      Transition battery lifecycle state
// @Description  Moves a battery through the FSM (e.g. OPERATIONAL → SECOND_LIFE)
// @Tags         lifecycle
// @Accept       json
// @Produce      json
// @Param        bpan  path   string                        true  "Battery PAN"
// @Param        body  body   models.TransitionStateRequest true  "Transition payload"
// @Success      200  {object}  map[string]interface{}
// @Failure      400  {object}  map[string]string
// @Failure      500  {object}  map[string]string
// @Router       /batteries/{bpan}/transition [post]
// @Security     BearerAuth
func TransitionState(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		bpan := chi.URLParam(r, "bpan")

		var req models.TransitionStateRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		actorID := middleware.GetUserID(r)
		actorRole := middleware.GetUserRole(r)

		resp, err := s.TransitionState(r.Context(), bpan, req.NewState, actorID, actorRole, req.Details)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}
		json.NewEncoder(w).Encode(resp)
	}
}

// ── TransferOwnership ─────────────────────────────────────────────────────────

// TransferOwnership godoc
// @Summary      Initiate battery ownership transfer
// @Description  Starts a dual-party consent ownership transfer
// @Tags         lifecycle
// @Accept       json
// @Produce      json
// @Param        bpan  path   string                              true  "Battery PAN"
// @Param        body  body   models.TransferInitiateRequest      true  "Transfer payload"
// @Success      202  {object}  map[string]string
// @Failure      400  {object}  map[string]string
// @Failure      500  {object}  map[string]string
// @Router       /batteries/{bpan}/ownership/transfer [post]
// @Security     BearerAuth
func TransferOwnership(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		bpan := chi.URLParam(r, "bpan")

		var req models.TransferInitiateRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		fromOwnerID := middleware.GetUserID(r)
		fromOwnerRole := middleware.GetUserRole(r)

		transferID, err := s.InitiateTransfer(r.Context(), bpan, fromOwnerID, fromOwnerRole, &req)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}

		w.WriteHeader(http.StatusAccepted)
		json.NewEncoder(w).Encode(map[string]string{
			"transfer_id": transferID,
			"status":      "pending_confirmation",
		})
	}
}

// ── ConfirmTransfer ───────────────────────────────────────────────────────────

// ConfirmTransfer godoc
// @Summary      Confirm an ownership transfer
// @Description  Confirms a pending transfer (either party). Transfer completes when both confirm.
// @Tags         lifecycle
// @Produce      json
// @Param        id  path  string  true  "Transfer ID"
// @Success      200  {object}  map[string]interface{}
// @Failure      500  {object}  map[string]string
// @Router       /ownership/transfer/{id}/confirm [post]
// @Security     BearerAuth
func ConfirmTransfer(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		transferID := chi.URLParam(r, "id")
		confirmingOwnerID := middleware.GetUserID(r)

		isComplete, err := s.ConfirmTransfer(r.Context(), transferID, confirmingOwnerID)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}
		json.NewEncoder(w).Encode(map[string]interface{}{
			"transfer_id": transferID,
			"is_complete": isComplete,
		})
	}
}

// ── RejectTransfer ────────────────────────────────────────────────────────────

// RejectTransfer godoc
// @Summary      Reject an ownership transfer
// @Description  Rejects a pending transfer (either party can reject)
// @Tags         lifecycle
// @Accept       json
// @Produce      json
// @Param        id    path   string  true  "Transfer ID"
// @Param        body  body   models.TransferRejectRequest  true  "Rejection reason"
// @Success      200  {object}  map[string]string
// @Failure      400  {object}  map[string]string
// @Failure      500  {object}  map[string]string
// @Router       /ownership/transfer/{id}/reject [post]
// @Security     BearerAuth
func RejectTransfer(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		transferID := chi.URLParam(r, "id")

		var req models.TransferRejectRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
			return
		}

		rejectingOwnerID := middleware.GetUserID(r)

		if err := s.RejectTransfer(r.Context(), transferID, rejectingOwnerID, req.Reason); err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}
		json.NewEncoder(w).Encode(map[string]string{
			"transfer_id": transferID,
			"status":      "rejected",
		})
	}
}

// ── GetOwnershipHistory ───────────────────────────────────────────────────────

// GetOwnershipHistory godoc
// @Summary      Get ownership history
// @Description  Returns the full chain of custody for a battery
// @Tags         lifecycle
// @Produce      json
// @Param        bpan  path  string  true  "Battery PAN"
// @Success      200  {array}   map[string]interface{}
// @Failure      501  {object}  map[string]string
// @Router       /batteries/{bpan}/ownership/history [get]
// @Security     BearerAuth
func GetOwnershipHistory(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		// TODO Day 13: wire to audit log query
		w.WriteHeader(http.StatusNotImplemented)
		json.NewEncoder(w).Encode(map[string]string{"error": "not_implemented"})
	}
}

// ── CertifyReuse ──────────────────────────────────────────────────────────────

// CertifyReuse godoc
// @Summary      Certify battery for second-life reuse
// @Description  Transitions state to SECOND_LIFE (reuse operator only)
// @Tags         lifecycle
// @Accept       json
// @Produce      json
// @Param        bpan  path  string  true  "Battery PAN"
// @Success      200  {object}  map[string]interface{}
// @Failure      500  {object}  map[string]string
// @Router       /batteries/{bpan}/reuse [post]
// @Security     BearerAuth
func CertifyReuse(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		bpan := chi.URLParam(r, "bpan")
		actorID := middleware.GetUserID(r)
		actorRole := middleware.GetUserRole(r)

		resp, err := s.TransitionState(r.Context(), bpan, "SECOND_LIFE", actorID, actorRole, "certified_for_reuse")
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}
		json.NewEncoder(w).Encode(resp)
	}
}

// ── RecordRecycling ───────────────────────────────────────────────────────────

// RecordRecycling godoc
// @Summary      Record battery recycling
// @Description  Transitions state to RECYCLED (recycler only)
// @Tags         lifecycle
// @Accept       json
// @Produce      json
// @Param        bpan  path  string  true  "Battery PAN"
// @Success      200  {object}  map[string]interface{}
// @Failure      500  {object}  map[string]string
// @Router       /batteries/{bpan}/recycling [post]
// @Security     BearerAuth
func RecordRecycling(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		bpan := chi.URLParam(r, "bpan")
		actorID := middleware.GetUserID(r)
		actorRole := middleware.GetUserRole(r)

		resp, err := s.TransitionState(r.Context(), bpan, "RECYCLED", actorID, actorRole, "recycling_recorded")
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}
		json.NewEncoder(w).Encode(resp)
	}
}

// ── VerifyOperational ─────────────────────────────────────────────────────────

// VerifyOperational godoc
// @Summary      Verify battery is operational (ZK proof)
// @Description  Generates a zero-knowledge proof that battery SoH meets threshold
// @Tags         lifecycle
// @Produce      json
// @Param        bpan  path  string  true  "Battery PAN"
// @Success      200  {object}  map[string]interface{}
// @Failure      500  {object}  map[string]string
// @Router       /batteries/{bpan}/verify/operational [post]
// @Security     BearerAuth
func VerifyOperational(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "service unavailable"})
			return
		}
		bpan := chi.URLParam(r, "bpan")
		requesterID := middleware.GetUserID(r)

		resp, err := s.VerifyOperational(r.Context(), bpan, requesterID)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}
		json.NewEncoder(w).Encode(resp)
	}
}

// ── VerifySignature ───────────────────────────────────────────────────────────

// VerifySignature godoc
// @Summary      Verify battery data signature
// @Description  Checks cryptographic integrity of all battery data
// @Tags         lifecycle
// @Produce      json
// @Param        bpan  path  string  true  "Battery PAN"
// @Success      200  {object}  map[string]interface{}
// @Failure      500  {object}  map[string]string
// @Router       /batteries/{bpan}/verify/signature [post]
// @Security     BearerAuth
func VerifySignature(s *services.LifecycleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		if s == nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"error": "service unavailable"})
			return
		}
		bpan := chi.URLParam(r, "bpan")

		resp, err := s.VerifySignature(r.Context(), bpan)
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
			return
		}
		json.NewEncoder(w).Encode(resp)
	}
}
