//! zk_proofs.rs — Zero-knowledge proof service trait
//!
//! All ZK operations are behind this trait so the gRPC handler layer
//! is never coupled to a specific proving system.
//! Concrete implementation (bulletproofs) lands on Day 12.

use std::fmt;

/// Opaque byte blob representing a serialised ZK proof.
/// Consumers must not interpret the bytes — use [`ZkProver::verify`].
#[derive(Debug, Clone, zeroize::Zeroize)]
pub struct ZkProof(pub Vec<u8>);

/// Opaque public inputs for a proof statement.
#[derive(Debug, Clone)]
pub struct ProofPublicInputs(pub Vec<u8>);

/// Error type for ZK operations.
#[derive(Debug)]
pub enum ZkError {
    /// The prover failed to generate a valid proof.
    ProvingFailed(String),
    /// A provided proof did not verify.
    VerificationFailed,
    /// Input value is outside the allowed range.
    OutOfRange { value: u64, min: u64, max: u64 },
    /// Internal error (e.g. RNG failure, serialisation error).
    Internal(String),
}

impl fmt::Display for ZkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZkError::ProvingFailed(msg)        => write!(f, "proving failed: {msg}"),
            ZkError::VerificationFailed        => write!(f, "proof verification failed"),
            ZkError::OutOfRange { value, min, max } =>
                write!(f, "value {value} is outside [{min}, {max}]"),
            ZkError::Internal(msg)             => write!(f, "internal ZK error: {msg}"),
        }
    }
}

impl std::error::Error for ZkError {}

/// The primary interface for all zero-knowledge operations in this service.
///
/// # Stability contract
/// Implementations must be deterministic given the same inputs and randomness
/// source so that proofs can be reproduced for audit purposes.
pub trait ZkProver: Send + Sync {
    /// Prove that `value` lies within `[min, max]` (inclusive) without
    /// revealing `value` to the verifier.
    ///
    /// Used for SoH range proofs:
    /// - `prove_range(soh, 81, 100)` → "battery is operational"
    /// - `prove_range(soh, 60, 80)`  → "battery is second-life eligible"
    fn prove_range(
        &self,
        value: u64,
        min:   u64,
        max:   u64,
    ) -> Result<(ZkProof, ProofPublicInputs), ZkError>;

    /// Verify a range proof produced by [`prove_range`].
    ///
    /// Returns `Ok(())` if the proof is valid, `Err(ZkError::VerificationFailed)`
    /// otherwise. Verifiers receive only the proof and public inputs — never
    /// the raw value.
    fn verify_range(
        &self,
        proof:  &ZkProof,
        public: &ProofPublicInputs,
        min:    u64,
        max:    u64,
    ) -> Result<(), ZkError>;

    /// Prove that a BPAN's static data has not been tampered with since
    /// manufacture, given the manufacturer's signature.
    ///
    /// Returns the proof and the public commitment (hash of signed data).
    fn prove_integrity(
        &self,
        bpan:      &str,
        data_hash: &[u8; 32],
        signature: &[u8],
    ) -> Result<(ZkProof, ProofPublicInputs), ZkError>;

    /// Verify an integrity proof.
    fn verify_integrity(
        &self,
        proof:  &ZkProof,
        public: &ProofPublicInputs,
    ) -> Result<(), ZkError>;
}
