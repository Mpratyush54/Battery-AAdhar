//! lifecycle.rs — Battery lifecycle state machine (FSM)
//!
//! States: REGISTERED → OPERATIONAL → SECOND_LIFE → EOL → RECYCLED → DESTROYED
//! Invalid transitions are rejected (e.g., RECYCLED → OPERATIONAL)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Battery lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleState {
    Registered,  // 1
    Operational, // 2
    SecondLife,  // 3
    EndOfLife,   // 4
    Recycled,    // 5
    Destroyed,   // 6
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LifecycleState::Registered => "REGISTERED",
            LifecycleState::Operational => "OPERATIONAL",
            LifecycleState::SecondLife => "SECOND_LIFE",
            LifecycleState::EndOfLife => "END_OF_LIFE",
            LifecycleState::Recycled => "RECYCLED",
            LifecycleState::Destroyed => "DESTROYED",
        };
        write!(f, "{s}")
    }
}

impl LifecycleState {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "REGISTERED" => Some(LifecycleState::Registered),
            "OPERATIONAL" => Some(LifecycleState::Operational),
            "SECOND_LIFE" => Some(LifecycleState::SecondLife),
            "END_OF_LIFE" => Some(LifecycleState::EndOfLife),
            "RECYCLED" => Some(LifecycleState::Recycled),
            "DESTROYED" => Some(LifecycleState::Destroyed),
            _ => None,
        }
    }

    /// Check if transition is valid
    pub fn can_transition_to(&self, next: LifecycleState) -> bool {
        match (self, &next) {
            // From REGISTERED
            (LifecycleState::Registered, LifecycleState::Operational) => true,

            // From OPERATIONAL
            (LifecycleState::Operational, LifecycleState::SecondLife) => true,
            (LifecycleState::Operational, LifecycleState::EndOfLife) => true,

            // From SECOND_LIFE
            (LifecycleState::SecondLife, LifecycleState::EndOfLife) => true,

            // From END_OF_LIFE
            (LifecycleState::EndOfLife, LifecycleState::Recycled) => true,

            // From RECYCLED
            (LifecycleState::Recycled, LifecycleState::Destroyed) => true,

            // Self-transitions allowed (e.g., OPERATIONAL -> OPERATIONAL for different ownership)
            (current, next) if current == next => true,

            // All other transitions invalid
            _ => false,
        }
    }
}

/// Ownership transfer (dual-party consent model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnershipTransfer {
    pub id: Uuid,
    pub bpan: String,
    pub from_owner_id: String,   // Approver
    pub to_owner_id: String,     // Acceptor
    pub from_owner_role: String, // "manufacturer", "distributor", etc.
    pub to_owner_role: String,
    pub transfer_reason: String, // "sale", "repair", "recycling", etc.

    // === Dual-Party Consent ===
    pub from_owner_confirmed: bool,
    pub from_owner_confirmed_at: Option<DateTime<Utc>>,
    pub to_owner_confirmed: bool,
    pub to_owner_confirmed_at: Option<DateTime<Utc>>,

    // === Rejection ===
    pub rejected: bool,
    pub rejection_reason: Option<String>,
    pub rejected_by: Option<String>,
    pub rejected_at: Option<DateTime<Utc>>,

    // === Metadata ===
    pub initiated_at: DateTime<Utc>,
    pub transferred_at: Option<DateTime<Utc>>,
}

impl OwnershipTransfer {
    pub fn new(
        bpan: String,
        from_owner_id: String,
        to_owner_id: String,
        from_owner_role: String,
        to_owner_role: String,
        reason: String,
    ) -> Self {
        OwnershipTransfer {
            id: Uuid::new_v4(),
            bpan,
            from_owner_id,
            to_owner_id,
            from_owner_role,
            to_owner_role,
            transfer_reason: reason,
            from_owner_confirmed: false,
            from_owner_confirmed_at: None,
            to_owner_confirmed: false,
            to_owner_confirmed_at: None,
            rejected: false,
            rejection_reason: None,
            rejected_by: None,
            rejected_at: None,
            initiated_at: Utc::now(),
            transferred_at: None,
        }
    }

    /// Both parties confirmed?
    pub fn is_complete(&self) -> bool {
        self.from_owner_confirmed && self.to_owner_confirmed && !self.rejected
    }

    /// Confirm by one party
    pub fn confirm(&mut self, owner_id: &str) -> Result<(), String> {
        if self.rejected {
            return Err("transfer already rejected".to_string());
        }

        if owner_id == self.from_owner_id {
            if self.from_owner_confirmed {
                return Err("already confirmed by from_owner".to_string());
            }
            self.from_owner_confirmed = true;
            self.from_owner_confirmed_at = Some(Utc::now());
        } else if owner_id == self.to_owner_id {
            if self.to_owner_confirmed {
                return Err("already confirmed by to_owner".to_string());
            }
            self.to_owner_confirmed = true;
            self.to_owner_confirmed_at = Some(Utc::now());
        } else {
            return Err("not a party to this transfer".to_string());
        }

        if self.is_complete() {
            self.transferred_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Reject by one party
    pub fn reject(&mut self, owner_id: &str, reason: String) -> Result<(), String> {
        if self.is_complete() {
            return Err("transfer already completed".to_string());
        }

        if owner_id != self.from_owner_id && owner_id != self.to_owner_id {
            return Err("not a party to this transfer".to_string());
        }

        self.rejected = true;
        self.rejection_reason = Some(reason);
        self.rejected_by = Some(owner_id.to_string());
        self.rejected_at = Some(Utc::now());

        Ok(())
    }
}

/// Ownership record (who currently owns battery)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryOwner {
    pub bpan: String,
    pub current_owner_id: String,
    pub current_owner_role: String,
    pub owned_since: DateTime<Utc>,
    pub owned_until: Option<DateTime<Utc>>,
}

/// Lifecycle event (transitions + transfers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub id: Uuid,
    pub bpan: String,
    pub event_type: String, // "STATE_TRANSITION", "OWNERSHIP_TRANSFER", "REUSE_CERT", "RECYCLING"
    pub from_state: Option<LifecycleState>,
    pub to_state: Option<LifecycleState>,
    pub actor_id: String,
    pub actor_role: String,
    pub details: String,         // JSON with event-specific data
    pub entry_hash: String,      // SHA256 for hash-chain
    pub entry_hash_prev: String, // Previous entry hash (for chain)
    pub created_at: DateTime<Utc>,
}

impl LifecycleEvent {
    pub fn new(
        bpan: String,
        event_type: String,
        from_state: Option<LifecycleState>,
        to_state: Option<LifecycleState>,
        actor_id: String,
        actor_role: String,
        details: String,
    ) -> Self {
        LifecycleEvent {
            id: Uuid::new_v4(),
            bpan,
            event_type,
            from_state,
            to_state,
            actor_id,
            actor_role,
            details,
            entry_hash: String::new(),        // Will be computed
            entry_hash_prev: "0".to_string(), // Will be set from DB
            created_at: Utc::now(),
        }
    }
}

/// Lifecycle history (timeline of all events)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleTimeline {
    pub bpan: String,
    pub current_state: LifecycleState,
    pub current_owner_id: String,
    pub events: Vec<LifecycleEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_valid_transitions() {
        assert!(LifecycleState::Registered.can_transition_to(LifecycleState::Operational));
        assert!(LifecycleState::Operational.can_transition_to(LifecycleState::SecondLife));
        assert!(LifecycleState::SecondLife.can_transition_to(LifecycleState::EndOfLife));
    }

    #[test]
    fn test_lifecycle_invalid_transitions() {
        // Cannot go backward
        assert!(!LifecycleState::Operational.can_transition_to(LifecycleState::Registered));
        // Cannot jump over states
        assert!(!LifecycleState::Registered.can_transition_to(LifecycleState::Recycled));
        // Cannot escape destruction
        assert!(!LifecycleState::Destroyed.can_transition_to(LifecycleState::Recycled));
    }

    #[test]
    fn test_ownership_transfer_dual_consent() {
        let mut transfer = OwnershipTransfer::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            "mfr-001".to_string(),
            "distributor-001".to_string(),
            "manufacturer".to_string(),
            "distributor".to_string(),
            "sale".to_string(),
        );

        assert!(!transfer.is_complete());

        // First party confirms
        transfer.confirm("mfr-001").unwrap();
        assert!(!transfer.is_complete()); // Waiting for second party

        // Second party confirms
        transfer.confirm("distributor-001").unwrap();
        assert!(transfer.is_complete()); // Transfer complete!
    }

    #[test]
    fn test_ownership_transfer_rejection() {
        let mut transfer = OwnershipTransfer::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            "mfr-001".to_string(),
            "distributor-001".to_string(),
            "manufacturer".to_string(),
            "distributor".to_string(),
            "sale".to_string(),
        );

        transfer.confirm("mfr-001").unwrap();

        // Second party rejects
        transfer
            .reject("distributor-001", "damaged goods".to_string())
            .unwrap();

        assert!(transfer.rejected);
        assert!(!transfer.is_complete());
    }

    #[test]
    fn test_ownership_transfer_invalid_actor() {
        let mut transfer = OwnershipTransfer::new(
            "MY008A6FKKKLC1DH80001".to_string(),
            "mfr-001".to_string(),
            "distributor-001".to_string(),
            "manufacturer".to_string(),
            "distributor".to_string(),
            "sale".to_string(),
        );

        // Invalid actor
        let result = transfer.confirm("other-party-001");
        assert!(result.is_err());
    }
}
