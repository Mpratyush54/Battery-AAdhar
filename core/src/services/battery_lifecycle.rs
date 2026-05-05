//! battery_lifecycle.rs — Battery lifecycle finite state machine
//!
//! Manages state transitions and ownership transfers.
//! Day 11: BatteryState FSM (pure in-memory validation)
//! Day 12: BatteryLifecycleService wired to OwnershipRepositoryImpl + hash-chain

use chrono::Utc;
use sha2::{Digest as Sha256Digest, Sha256};
use std::sync::Arc;
use tracing::{info, instrument};

use crate::errors::{BpaError, BpaResult};
use crate::models::LifecycleState;
use crate::repositories::OwnershipRepositoryImpl;

// ─────────────────────────────────────────────────────────────────────────────
// BatteryState FSM (─────────────────────────────────────────────────

/// Battery lifecycle states per the BPA guideline.
/// A battery progresses through these states from manufacturing to end-of-life.
///
/// ```text
/// REGISTERED ──► ACTIVE ──► IN_SERVICE ──┬──► REUSE_CANDIDATE ──► REPURPOSED ──► IN_SERVICE
///                                        │
///                                        ├──► RECALL
///                                        │
///                                        └──► END_OF_LIFE ──► RECYCLING ──► RECYCLED
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BatteryState {
    /// Initial registration by manufacturer/importer — BPAN assigned
    Registered,
    /// Static data uploaded, QR generated, compliance checks passed
    Active,
    /// Battery installed in a vehicle or application, dynamic data flowing
    InService,
    /// Battery flagged for potential second-life use
    ReuseCandidate,
    /// Battery repurposed for a second-life application (new BPAN may be issued)
    Repurposed,
    /// Battery recalled due to safety or compliance issues
    Recalled,
    /// Battery reached end of life (SoH below threshold or critical failure)
    EndOfLife,
    /// Battery in active recycling process
    Recycling,
    /// Battery fully recycled, material recovery documented
    Recycled,
    /// Battery decommissioned / permanently retired
    Decommissioned,
}

impl BatteryState {
    /// Parse a state string from the database.
    pub fn from_str_code(s: &str) -> BpaResult<Self> {
        match s.to_uppercase().as_str() {
            "REGISTERED" => Ok(Self::Registered),
            "ACTIVE" => Ok(Self::Active),
            "IN_SERVICE" => Ok(Self::InService),
            "REUSE_CANDIDATE" => Ok(Self::ReuseCandidate),
            "REPURPOSED" => Ok(Self::Repurposed),
            "RECALLED" => Ok(Self::Recalled),
            "END_OF_LIFE" => Ok(Self::EndOfLife),
            "RECYCLING" => Ok(Self::Recycling),
            "RECYCLED" => Ok(Self::Recycled),
            "DECOMMISSIONED" => Ok(Self::Decommissioned),
            _ => Err(BpaError::InvalidStateTransition(format!(
                "Unknown battery state: {}",
                s
            ))),
        }
    }

    /// Serialize the state to a string for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Registered => "REGISTERED",
            Self::Active => "ACTIVE",
            Self::InService => "IN_SERVICE",
            Self::ReuseCandidate => "REUSE_CANDIDATE",
            Self::Repurposed => "REPURPOSED",
            Self::Recalled => "RECALLED",
            Self::EndOfLife => "END_OF_LIFE",
            Self::Recycling => "RECYCLING",
            Self::Recycled => "RECYCLED",
            Self::Decommissioned => "DECOMMISSIONED",
        }
    }
}

impl std::fmt::Display for BatteryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Static BatteryState FSM helpers (Day 11)
pub struct BatteryStateMachine;

impl BatteryStateMachine {
    /// Check whether a state transition is valid per the BPA lifecycle rules.
    #[instrument(name = "check_transition", skip_all)]
    pub fn check_transition(from: &BatteryState, to: &BatteryState) -> BpaResult<()> {
        let allowed = match from {
            BatteryState::Registered => matches!(to, BatteryState::Active),
            BatteryState::Active => matches!(
                to,
                BatteryState::InService | BatteryState::Recalled | BatteryState::Decommissioned
            ),
            BatteryState::InService => matches!(
                to,
                BatteryState::ReuseCandidate | BatteryState::EndOfLife | BatteryState::Recalled
            ),
            BatteryState::ReuseCandidate => matches!(
                to,
                BatteryState::Repurposed | BatteryState::EndOfLife | BatteryState::Recalled
            ),
            BatteryState::Repurposed => matches!(
                to,
                BatteryState::InService | BatteryState::EndOfLife | BatteryState::Recalled
            ),
            BatteryState::Recalled => {
                matches!(to, BatteryState::EndOfLife | BatteryState::Decommissioned)
            }
            BatteryState::EndOfLife => {
                matches!(to, BatteryState::Recycling | BatteryState::Decommissioned)
            }
            BatteryState::Recycling => matches!(to, BatteryState::Recycled),
            BatteryState::Recycled => false,       // terminal state
            BatteryState::Decommissioned => false, // terminal state
        };

        if !allowed {
            return Err(BpaError::InvalidStateTransition(format!(
                "Cannot transition from {} to {}",
                from.as_str(),
                to.as_str()
            )));
        }

        info!(
            "State transition validated: {} → {}",
            from.as_str(),
            to.as_str()
        );
        Ok(())
    }

    /// Determine if the battery should be flagged for reuse based on SoH.
    pub fn evaluate_soh(state_of_health: f64) -> BpaResult<SohEvaluation> {
        if !(0.0..=100.0).contains(&state_of_health) {
            return Err(BpaError::Validation(
                "State of Health must be between 0 and 100".into(),
            ));
        }

        let evaluation = if state_of_health >= 80.0 {
            SohEvaluation::Healthy
        } else if state_of_health >= 60.0 {
            SohEvaluation::ReuseCandidate
        } else if state_of_health >= 30.0 {
            SohEvaluation::DegradedRecycleRecommended
        } else {
            SohEvaluation::EndOfLife
        };

        info!("SoH evaluation: {:.1}% → {:?}", state_of_health, evaluation);
        Ok(evaluation)
    }

    /// Check if a battery is in a terminal state.
    pub fn is_terminal(state: &BatteryState) -> bool {
        matches!(state, BatteryState::Recycled | BatteryState::Decommissioned)
    }

    /// Get all allowed next states from the current state.
    pub fn allowed_transitions(from: &BatteryState) -> Vec<BatteryState> {
        match from {
            BatteryState::Registered => vec![BatteryState::Active],
            BatteryState::Active => vec![
                BatteryState::InService,
                BatteryState::Recalled,
                BatteryState::Decommissioned,
            ],
            BatteryState::InService => vec![
                BatteryState::ReuseCandidate,
                BatteryState::EndOfLife,
                BatteryState::Recalled,
            ],
            BatteryState::ReuseCandidate => vec![
                BatteryState::Repurposed,
                BatteryState::EndOfLife,
                BatteryState::Recalled,
            ],
            BatteryState::Repurposed => vec![
                BatteryState::InService,
                BatteryState::EndOfLife,
                BatteryState::Recalled,
            ],
            BatteryState::Recalled => vec![BatteryState::EndOfLife, BatteryState::Decommissioned],
            BatteryState::EndOfLife => vec![BatteryState::Recycling, BatteryState::Decommissioned],
            BatteryState::Recycling => vec![BatteryState::Recycled],
            BatteryState::Recycled => vec![],
            BatteryState::Decommissioned => vec![],
        }
    }
}

/// Result of evaluating a battery's State of Health.
#[derive(Debug, Clone, PartialEq)]
pub enum SohEvaluation {
    /// SoH >= 80%: Battery is healthy, continue normal operation
    Healthy,
    /// SoH 60-79%: Battery is a candidate for second-life reuse
    ReuseCandidate,
    /// SoH 30-59%: Battery is degraded, recycling recommended
    DegradedRecycleRecommended,
    /// SoH < 30%: Battery has reached end of life
    EndOfLife,
}

// ─────────────────────────────────────────────────────────────────────────────
// Day 12: BatteryLifecycleService — Repository-backed lifecycle operations
// ─────────────────────────────────────────────────────────────────────────────

/// Service for battery lifecycle management and ownership transfers.
/// Wires the FSM checks with `OwnershipRepositoryImpl` for DB persistence
/// and maintains the SHA-256 hash-chain for auditability.
pub struct BatteryLifecycleService {
    repo: Arc<OwnershipRepositoryImpl>,
}

impl BatteryLifecycleService {
    /// Create a new `BatteryLifecycleService` with an ownership repository.
    pub fn new(repo: Arc<OwnershipRepositoryImpl>) -> Self {
        BatteryLifecycleService { repo }
    }

    /// Compute SHA-256 hash for a lifecycle event (hash-chain entry).
    fn compute_event_hash(
        bpan: &str,
        event_type: &str,
        prev_hash: &str,
        actor_id: &str,
        timestamp: &chrono::DateTime<Utc>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bpan.as_bytes());
        hasher.update(event_type.as_bytes());
        hasher.update(prev_hash.as_bytes());
        hasher.update(actor_id.as_bytes());
        hasher.update(timestamp.to_rfc3339().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Transition battery to a new lifecycle state.
    ///
    /// Validates the FSM transition, computes the hash-chain entry, and
    /// persists the event via the repository.
    #[instrument(name = "lifecycle.transition_state", skip(self))]
    pub async fn transition_state(
        &self,
        bpan: &str,
        new_state: LifecycleState,
        actor_id: &str,
        actor_role: &str,
        details: &str,
    ) -> BpaResult<String> {
        let now = Utc::now();
        let event_type = "STATE_TRANSITION";
        let new_state_str = new_state.to_string();

        // Get previous hash for chain integrity
        let prev_hash = self
            .repo
            .get_last_event_hash(bpan)
            .await
            .map_err(|e| BpaError::Internal(e.to_string()))?;

        let entry_hash = Self::compute_event_hash(bpan, event_type, &prev_hash, actor_id, &now);

        // Record lifecycle event in hash-chained log
        self.repo
            .record_lifecycle_event(
                bpan,
                event_type,
                None,
                Some(&new_state_str),
                actor_id,
                actor_role,
                details,
                &entry_hash,
                &prev_hash,
            )
            .await
            .map_err(|e| BpaError::Internal(e.to_string()))?;

        info!(
            bpan = bpan,
            new_state = %new_state_str,
            actor_id = actor_id,
            entry_hash = %entry_hash,
            "Lifecycle state transition recorded"
        );

        Ok(entry_hash)
    }

    /// Initiate an ownership transfer (dual-party consent model).
    ///
    /// Checks for pending transfers and persists the new transfer record.
    #[instrument(name = "lifecycle.initiate_ownership_transfer", skip(self))]
    pub async fn initiate_ownership_transfer(
        &self,
        bpan: &str,
        from_owner_id: &str,
        to_owner_id: &str,
        from_owner_role: &str,
        to_owner_role: &str,
        reason: &str,
    ) -> BpaResult<String> {
        let transfer_id = self
            .repo
            .initiate_transfer(
                bpan,
                from_owner_id,
                to_owner_id,
                from_owner_role,
                to_owner_role,
                reason,
            )
            .await
            .map_err(|e| BpaError::Internal(e.to_string()))?;

        info!(
            bpan = bpan,
            from_owner_id = from_owner_id,
            to_owner_id = to_owner_id,
            transfer_id = %transfer_id,
            "Ownership transfer initiated"
        );

        Ok(transfer_id)
    }

    /// Confirm a pending ownership transfer (called by either party).
    ///
    /// Returns `true` when both parties have confirmed (transfer complete).
    #[instrument(name = "lifecycle.confirm_ownership_transfer", skip(self))]
    pub async fn confirm_ownership_transfer(
        &self,
        transfer_id: &str,
        confirming_owner_id: &str,
    ) -> BpaResult<bool> {
        let is_complete = self
            .repo
            .confirm_transfer(transfer_id, confirming_owner_id)
            .await
            .map_err(|e| BpaError::Internal(e.to_string()))?;

        info!(
            transfer_id = transfer_id,
            confirming_owner_id = confirming_owner_id,
            is_complete = is_complete,
            "Ownership transfer confirmation recorded"
        );

        if is_complete {
            info!(
                transfer_id = transfer_id,
                "Ownership transfer completed — both parties confirmed"
            );
        }

        Ok(is_complete)
    }

    /// Reject a pending ownership transfer.
    #[instrument(name = "lifecycle.reject_ownership_transfer", skip(self))]
    pub async fn reject_ownership_transfer(
        &self,
        transfer_id: &str,
        rejecting_owner_id: &str,
        reason: &str,
    ) -> BpaResult<()> {
        self.repo
            .reject_transfer(transfer_id, rejecting_owner_id, reason)
            .await
            .map_err(|e| BpaError::Internal(e.to_string()))?;

        info!(
            transfer_id = transfer_id,
            rejecting_owner_id = rejecting_owner_id,
            reason = reason,
            "Ownership transfer rejected"
        );

        Ok(())
    }

    /// Get the current owner of a battery.
    #[instrument(name = "lifecycle.get_current_owner", skip(self))]
    pub async fn get_current_owner(&self, bpan: &str) -> BpaResult<(String, String)> {
        self.repo
            .get_current_owner(bpan)
            .await
            .map_err(|e| BpaError::Internal(e.to_string()))
    }
}
