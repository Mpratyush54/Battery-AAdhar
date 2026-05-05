// lifecycle_day12.go — Hand-written supplement for Day 12 proto additions.
// The upstream lifecycle.pb.go / lifecycle_grpc.pb.go were generated before
// the 4 new RPCs were added to lifecycle.proto.  Rather than regenerating
// (which requires protoc toolchain), we define the missing types here so the
// Go service layer can compile.  When the toolchain is available, delete this
// file and regenerate from proto/lifecycle.proto instead.

package lifecyclev1

// ── TransitionState ──────────────────────────────────────────────────────────

type TransitionStateRequest struct {
	Bpan      string `protobuf:"bytes,1,opt,name=bpan,proto3" json:"bpan,omitempty"`
	NewState  string `protobuf:"bytes,2,opt,name=new_state,json=newState,proto3" json:"new_state,omitempty"`
	ActorId   string `protobuf:"bytes,3,opt,name=actor_id,json=actorId,proto3" json:"actor_id,omitempty"`
	ActorRole string `protobuf:"bytes,4,opt,name=actor_role,json=actorRole,proto3" json:"actor_role,omitempty"`
	Details   string `protobuf:"bytes,5,opt,name=details,proto3" json:"details,omitempty"`
}

func (x *TransitionStateRequest) Reset()         {}
func (x *TransitionStateRequest) String() string { return x.Bpan + "→" + x.NewState }
func (x *TransitionStateRequest) ProtoMessage()  {}

func (x *TransitionStateRequest) GetBpan() string      { return x.Bpan }
func (x *TransitionStateRequest) GetNewState() string  { return x.NewState }
func (x *TransitionStateRequest) GetActorId() string   { return x.ActorId }
func (x *TransitionStateRequest) GetActorRole() string { return x.ActorRole }
func (x *TransitionStateRequest) GetDetails() string   { return x.Details }

type TransitionStateResponse struct {
	Success   bool   `protobuf:"varint,1,opt,name=success,proto3" json:"success,omitempty"`
	EventId   string `protobuf:"bytes,2,opt,name=event_id,json=eventId,proto3" json:"event_id,omitempty"`
	EntryHash string `protobuf:"bytes,3,opt,name=entry_hash,json=entryHash,proto3" json:"entry_hash,omitempty"`
}

func (x *TransitionStateResponse) Reset()         {}
func (x *TransitionStateResponse) String() string { return x.EventId }
func (x *TransitionStateResponse) ProtoMessage()  {}

func (x *TransitionStateResponse) GetSuccess() bool     { return x.Success }
func (x *TransitionStateResponse) GetEventId() string   { return x.EventId }
func (x *TransitionStateResponse) GetEntryHash() string { return x.EntryHash }

// ── InitiateTransfer ─────────────────────────────────────────────────────────

type InitiateTransferRequest struct {
	Bpan          string `protobuf:"bytes,1,opt,name=bpan,proto3" json:"bpan,omitempty"`
	FromOwnerId   string `protobuf:"bytes,2,opt,name=from_owner_id,json=fromOwnerId,proto3" json:"from_owner_id,omitempty"`
	ToOwnerId     string `protobuf:"bytes,3,opt,name=to_owner_id,json=toOwnerId,proto3" json:"to_owner_id,omitempty"`
	FromOwnerRole string `protobuf:"bytes,4,opt,name=from_owner_role,json=fromOwnerRole,proto3" json:"from_owner_role,omitempty"`
	ToOwnerRole   string `protobuf:"bytes,5,opt,name=to_owner_role,json=toOwnerRole,proto3" json:"to_owner_role,omitempty"`
	Reason        string `protobuf:"bytes,6,opt,name=reason,proto3" json:"reason,omitempty"`
}

func (x *InitiateTransferRequest) Reset()         {}
func (x *InitiateTransferRequest) String() string { return x.Bpan }
func (x *InitiateTransferRequest) ProtoMessage()  {}

func (x *InitiateTransferRequest) GetBpan() string          { return x.Bpan }
func (x *InitiateTransferRequest) GetFromOwnerId() string   { return x.FromOwnerId }
func (x *InitiateTransferRequest) GetToOwnerId() string     { return x.ToOwnerId }
func (x *InitiateTransferRequest) GetFromOwnerRole() string { return x.FromOwnerRole }
func (x *InitiateTransferRequest) GetToOwnerRole() string   { return x.ToOwnerRole }
func (x *InitiateTransferRequest) GetReason() string        { return x.Reason }

type InitiateTransferResponse struct {
	TransferId string `protobuf:"bytes,1,opt,name=transfer_id,json=transferId,proto3" json:"transfer_id,omitempty"`
}

func (x *InitiateTransferResponse) Reset()         {}
func (x *InitiateTransferResponse) String() string { return x.TransferId }
func (x *InitiateTransferResponse) ProtoMessage()  {}

func (x *InitiateTransferResponse) GetTransferId() string { return x.TransferId }

// ── ConfirmTransfer ──────────────────────────────────────────────────────────

type ConfirmTransferRequest struct {
	TransferId        string `protobuf:"bytes,1,opt,name=transfer_id,json=transferId,proto3" json:"transfer_id,omitempty"`
	ConfirmingOwnerId string `protobuf:"bytes,2,opt,name=confirming_owner_id,json=confirmingOwnerId,proto3" json:"confirming_owner_id,omitempty"`
}

func (x *ConfirmTransferRequest) Reset()         {}
func (x *ConfirmTransferRequest) String() string { return x.TransferId }
func (x *ConfirmTransferRequest) ProtoMessage()  {}

func (x *ConfirmTransferRequest) GetTransferId() string        { return x.TransferId }
func (x *ConfirmTransferRequest) GetConfirmingOwnerId() string { return x.ConfirmingOwnerId }

type ConfirmTransferResponse struct {
	IsComplete bool `protobuf:"varint,1,opt,name=is_complete,json=isComplete,proto3" json:"is_complete,omitempty"`
}

func (x *ConfirmTransferResponse) Reset()         {}
func (x *ConfirmTransferResponse) String() string { return "" }
func (x *ConfirmTransferResponse) ProtoMessage()  {}

func (x *ConfirmTransferResponse) GetIsComplete() bool { return x.IsComplete }

// ── RejectTransfer ───────────────────────────────────────────────────────────

type RejectTransferRequest struct {
	TransferId       string `protobuf:"bytes,1,opt,name=transfer_id,json=transferId,proto3" json:"transfer_id,omitempty"`
	RejectingOwnerId string `protobuf:"bytes,2,opt,name=rejecting_owner_id,json=rejectingOwnerId,proto3" json:"rejecting_owner_id,omitempty"`
	Reason           string `protobuf:"bytes,3,opt,name=reason,proto3" json:"reason,omitempty"`
}

func (x *RejectTransferRequest) Reset()         {}
func (x *RejectTransferRequest) String() string { return x.TransferId }
func (x *RejectTransferRequest) ProtoMessage()  {}

func (x *RejectTransferRequest) GetTransferId() string       { return x.TransferId }
func (x *RejectTransferRequest) GetRejectingOwnerId() string { return x.RejectingOwnerId }
func (x *RejectTransferRequest) GetReason() string           { return x.Reason }

type RejectTransferResponse struct {
	Success bool `protobuf:"varint,1,opt,name=success,proto3" json:"success,omitempty"`
}

func (x *RejectTransferResponse) Reset()         {}
func (x *RejectTransferResponse) String() string { return "" }
func (x *RejectTransferResponse) ProtoMessage()  {}

func (x *RejectTransferResponse) GetSuccess() bool { return x.Success }
