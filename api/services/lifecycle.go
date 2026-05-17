// lifecycle.go — Battery lifecycle service (Go side)
// Orchestrates between HTTP handlers and Rust gRPC backend.

package services

import (
	"context"
	"fmt"
	"log/slog"

	lifecyclev1 "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/lifecycle/v1"
	"github.com/Mpratyush54/Battery-AAdhar/api/grpc"
	"github.com/Mpratyush54/Battery-AAdhar/api/models"
)

type LifecycleService struct {
	lifecycleClient lifecyclev1.LifecycleServiceClient
}

func NewLifecycleService(cc *grpc.ClientConn) *LifecycleService {
	return &LifecycleService{
		lifecycleClient: cc.LifecycleClient,
	}
}

// TransitionState handles FSM state changes (e.g. OPERATIONAL -> REUSE)
func (s *LifecycleService) TransitionState(
	ctx context.Context,
	bpan string,
	newState string,
	actorID string,
	actorRole string,
	details string,
) (*lifecyclev1.TransitionStateResponse, error) {
	slog.Info("requesting lifecycle state transition",
		"bpan", bpan,
		"new_state", newState,
		"actor", actorID,
	)

	resp, err := s.lifecycleClient.TransitionState(ctx, &lifecyclev1.TransitionStateRequest{
		Bpan:      bpan,
		NewState:  newState,
		ActorId:   actorID,
		ActorRole: actorRole,
		Details:   details,
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}

	return resp, nil
}

// InitiateTransfer starts a dual-party ownership transfer
func (s *LifecycleService) InitiateTransfer(
	ctx context.Context,
	bpan string,
	fromOwnerID string,
	fromOwnerRole string,
	req *models.TransferInitiateRequest,
) (string, error) {
	slog.Info("initiating ownership transfer",
		"bpan", bpan,
		"from", fromOwnerID,
		"to", req.ToOwnerId,
	)

	resp, err := s.lifecycleClient.InitiateTransfer(ctx, &lifecyclev1.InitiateTransferRequest{
		Bpan:          bpan,
		FromOwnerId:   fromOwnerID,
		ToOwnerId:     req.ToOwnerId,
		FromOwnerRole: fromOwnerRole,
		ToOwnerRole:   req.ToOwnerRole,
		Reason:        req.Reason,
	})
	if err != nil {
		return "", fmt.Errorf("gRPC error: %w", err)
	}

	return resp.TransferId, nil
}

// ConfirmTransfer confirms a pending transfer (usually called by the receiver)
func (s *LifecycleService) ConfirmTransfer(
	ctx context.Context,
	transferID string,
	confirmingOwnerID string,
) (bool, error) {
	slog.Info("confirming ownership transfer",
		"transfer_id", transferID,
		"confirmer", confirmingOwnerID,
	)

	resp, err := s.lifecycleClient.ConfirmTransfer(ctx, &lifecyclev1.ConfirmTransferRequest{
		TransferId:        transferID,
		ConfirmingOwnerId: confirmingOwnerID,
	})
	if err != nil {
		return false, fmt.Errorf("gRPC error: %w", err)
	}

	return resp.IsComplete, nil
}

// RejectTransfer rejects a pending transfer
func (s *LifecycleService) RejectTransfer(
	ctx context.Context,
	transferID string,
	rejectingOwnerID string,
	reason string,
) error {
	slog.Info("rejecting ownership transfer",
		"transfer_id", transferID,
		"rejecter", rejectingOwnerID,
	)

	resp, err := s.lifecycleClient.RejectTransfer(ctx, &lifecyclev1.RejectTransferRequest{
		TransferId:       transferID,
		RejectingOwnerId: rejectingOwnerID,
		Reason:           reason,
	})
	if err != nil {
		return fmt.Errorf("gRPC error: %w", err)
	}

	if !resp.Success {
		return fmt.Errorf("rejection failed on server")
	}

	return nil
}

// VerifyOperational calls Rust ZK prover to verify battery health state
func (s *LifecycleService) VerifyOperational(
	ctx context.Context,
	bpan string,
	requesterID string,
) (*lifecyclev1.VerifyOperationalResponse, error) {
	resp, err := s.lifecycleClient.VerifyOperational(ctx, &lifecyclev1.VerifyOperationalRequest{
		Bpan:        bpan,
		RequesterId: requesterID,
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}
	return resp, nil
}

// VerifySignature checks cryptographic integrity of battery data
func (s *LifecycleService) VerifySignature(
	ctx context.Context,
	bpan string,
) (*lifecyclev1.VerifySignatureResponse, error) {
	resp, err := s.lifecycleClient.VerifySignature(ctx, &lifecyclev1.VerifySignatureRequest{
		Bpan: bpan,
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}
	return resp, nil
}

// GetOwnershipHistory returns the full chain of custody for a battery
func (s *LifecycleService) GetOwnershipHistory(
	ctx context.Context,
	bpan string,
) (*lifecyclev1.GetOwnershipHistoryResponse, error) {
	resp, err := s.lifecycleClient.GetOwnershipHistory(ctx, &lifecyclev1.GetOwnershipHistoryRequest{
		Bpan: bpan,
	})
	if err != nil {
		return nil, fmt.Errorf("gRPC error: %w", err)
	}
	return resp, nil
}
