use tracing::{info, instrument};

use crate::errors::{BpaError, BpaResult};

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

pub struct BatteryLifecycleService;

impl BatteryLifecycleService {
    /// Check whether a state transition is valid per the BPA lifecycle rules.
    /// Returns Ok(()) if valid, Err with reason if not allowed.
    #[instrument(name = "check_transition", skip_all)]
    pub fn check_transition(from: &BatteryState, to: &BatteryState) -> BpaResult<()> {
        let allowed = match from {
            BatteryState::Registered => matches!(to, BatteryState::Active),
            BatteryState::Active => matches!(to, BatteryState::InService | BatteryState::Recalled | BatteryState::Decommissioned),
            BatteryState::InService => matches!(
                to,
                BatteryState::ReuseCandidate
                    | BatteryState::EndOfLife
                    | BatteryState::Recalled
            ),
            BatteryState::ReuseCandidate => matches!(
                to,
                BatteryState::Repurposed | BatteryState::EndOfLife | BatteryState::Recalled
            ),
            BatteryState::Repurposed => matches!(
                to,
                BatteryState::InService | BatteryState::EndOfLife | BatteryState::Recalled
            ),
            BatteryState::Recalled => matches!(
                to,
                BatteryState::EndOfLife | BatteryState::Decommissioned
            ),
            BatteryState::EndOfLife => matches!(to, BatteryState::Recycling | BatteryState::Decommissioned),
            BatteryState::Recycling => matches!(to, BatteryState::Recycled),
            BatteryState::Recycled => false,        // terminal state
            BatteryState::Decommissioned => false,  // terminal state
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
    /// Per BPA guidelines, batteries with SoH between 60-80% are reuse candidates.
    pub fn evaluate_soh(state_of_health: f64) -> BpaResult<SohEvaluation> {
        if state_of_health < 0.0 || state_of_health > 100.0 {
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

        info!(
            "SoH evaluation: {:.1}% → {:?}",
            state_of_health, evaluation
        );
        Ok(evaluation)
    }

    /// Check if a battery is in a terminal state (no further transitions possible).
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
            BatteryState::Recalled => vec![
                BatteryState::EndOfLife,
                BatteryState::Decommissioned,
            ],
            BatteryState::EndOfLife => vec![
                BatteryState::Recycling,
                BatteryState::Decommissioned,
            ],
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
