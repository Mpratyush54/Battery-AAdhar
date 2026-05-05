// lifecycle.go — Lifecycle models

package models

import "time"

type TransferInitiateRequest struct {
	ToOwnerId   string `json:"to_owner_id"`
	ToOwnerRole string `json:"to_owner_role"`
	Reason      string `json:"reason"`
}

type TransitionStateRequest struct {
	NewState string `json:"new_state"`
	Details  string `json:"details"`
}

type TransferRejectRequest struct {
	Reason string `json:"reason"`
}

type LifecycleEvent struct {
	EventID   string    `json:"event_id"`
	BPAN      string    `json:"bpan"`
	EventType string    `json:"event_type"` // STATE_TRANSITION, OWNERSHIP_TRANSFER, etc.
	FromState *string   `json:"from_state,omitempty"`
	ToState   *string   `json:"to_state,omitempty"`
	ActorID   string    `json:"actor_id"`
	ActorRole string    `json:"actor_role"`
	Details   string    `json:"details"`
	CreatedAt time.Time `json:"created_at"`
}

type LifecycleTimeline struct {
	BPAN         string           `json:"bpan"`
	CurrentState string           `json:"current_state"`
	CurrentOwner string           `json:"current_owner"`
	Events       []LifecycleEvent `json:"events"`
}
