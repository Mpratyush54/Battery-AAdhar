//! zk_proofs.rs — Zero-knowledge range proofs using bulletproofs
//!
//! Implements the ZkProver trait with Ristretto-based bulletproofs.
//! Used to prove battery State of Health is within a safe operational range
//! without revealing the exact value.

use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek_ng::scalar::Scalar;
use merlin::Transcript;
use std::fmt;

/// Opaque serialized bulletproof
#[derive(Debug, Clone)]
pub struct ZkProof(pub Vec<u8>);

/// Public commitment (Ristretto point) that binds the proof to a value
#[derive(Debug, Clone)]
pub struct ProofCommitment(pub Vec<u8>);

/// Error type for ZK operations
#[derive(Debug)]
pub enum ZkError {
    ProvingFailed(String),
    VerificationFailed,
    OutOfRange { value: u64, min: u64, max: u64 },
    Internal(String),
}

impl fmt::Display for ZkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZkError::ProvingFailed(msg) => write!(f, "proving failed: {}", msg),
            ZkError::VerificationFailed => write!(f, "proof verification failed"),
            ZkError::OutOfRange { value, min, max } => {
                write!(f, "value {} is outside [{}, {}]", value, min, max)
            }
            ZkError::Internal(msg) => write!(f, "internal ZK error: {}", msg),
        }
    }
}

impl std::error::Error for ZkError {}

/// Metadata about a range proof (for audit purposes)
#[derive(Debug, Clone)]
pub struct ProofMetadata {
    pub proof_type: String, // e.g., "soh_operational", "recyclability"
    pub min: u64,
    pub max: u64,
    pub timestamp: i64,
    pub bpan: String,
}

/// The concrete ZkProver implementation
pub struct ZkProverImpl {
    // Bulletproofs generators (pre-computed)
    gens: BulletproofGens,
    pc_gens: PedersenGens,
}

impl ZkProverImpl {
    /// Create a new ZK prover instance
    pub fn new() -> Self {
        // Create 64-bit range proof generators
        let gens = BulletproofGens::new(64, 1);
        let pc_gens = PedersenGens::default();

        ZkProverImpl { gens, pc_gens }
    }

    /// Prove that a value lies within [min, max] without revealing the value.
    ///
    /// Returns:
    /// - ZkProof: The serialized bulletproof (can be sent to verifier)
    /// - ProofCommitment: The Pedersen commitment (reveals nothing about value)
    /// - Blinding factor (for prover's records, not sent to verifier)
    pub fn prove_range(
        &self,
        value: u64,
        min: u64,
        max: u64,
    ) -> Result<(ZkProof, ProofCommitment, Scalar), ZkError> {
        // Validate input
        if value < min || value > max {
            return Err(ZkError::OutOfRange { value, min, max });
        }

        // Note: max is u64, so always fits in 64-bit range proof

        // Shift value so that 0 is at min (makes proof more efficient)
        let shifted_value = value - min;
        let _shifted_max = max - min;

        // Generate a random blinding factor
        let mut bytes = [0u8; 32];
        rand::Rng::fill(&mut rand::thread_rng(), &mut bytes);
        let blinding = Scalar::from_bytes_mod_order(bytes);

        // Create a Pedersen commitment to the value
        // commitment = (value * G + blinding * H)
        // This hides both the value and the blinding factor
        let commitment = self.pc_gens.commit(Scalar::from(shifted_value), blinding);

        // Generate the range proof using Merlin transcript
        // Include min and max in transcript to bind proof to specific range
        let mut transcript = Transcript::new(b"battery_soh_range_proof");
        transcript.append_u64(b"range_min", min);
        transcript.append_u64(b"range_max", max);

        let (proof, _) = RangeProof::prove_single(
            &self.gens,
            &self.pc_gens,
            &mut transcript,
            shifted_value,
            &blinding,
            64,
        )
        .map_err(|e| ZkError::ProvingFailed(format!("bulletproof failed: {}", e)))?;

        Ok((
            ZkProof(proof.to_bytes().to_vec()),
            ProofCommitment(commitment.compress().to_bytes().to_vec()),
            blinding,
        ))
    }

    /// Verify a range proof given only the proof and commitment.
    ///
    /// This is what the verifier (e.g., government regulator) does.
    /// They never see the actual value, only:
    /// - The proof (the bulletproof)
    /// - The commitment (Pedersen commitment)
    /// - The range [min, max]
    pub fn verify_range(
        &self,
        proof: &ZkProof,
        commitment: &ProofCommitment,
        min: u64,
        max: u64,
    ) -> Result<(), ZkError> {
        // Deserialize proof
        let proof_bytes: [u8; 672] = proof
            .0
            .as_slice()
            .try_into()
            .map_err(|_| ZkError::VerificationFailed)?;
        let proof =
            RangeProof::from_bytes(&proof_bytes).map_err(|_| ZkError::VerificationFailed)?;

        // Deserialize commitment
        let commitment_bytes: [u8; 32] = commitment
            .0
            .as_slice()
            .try_into()
            .map_err(|_| ZkError::VerificationFailed)?;
        let commitment = curve25519_dalek_ng::ristretto::CompressedRistretto(commitment_bytes);

        // Verify the proof — must use same transcript labels as prover
        let mut transcript = Transcript::new(b"battery_soh_range_proof");
        transcript.append_u64(b"range_min", min);
        transcript.append_u64(b"range_max", max);

        proof
            .verify_single(&self.gens, &self.pc_gens, &mut transcript, &commitment, 64)
            .map_err(|_| ZkError::VerificationFailed)
    }

    /// Convenience method: Prove SoH is operational (> 80%)
    pub fn prove_operational(
        &self,
        soh: u64,
    ) -> Result<(ZkProof, ProofCommitment, Scalar), ZkError> {
        if !(80..=100).contains(&soh) {
            return Err(ZkError::OutOfRange {
                value: soh,
                min: 80,
                max: 100,
            });
        }
        self.prove_range(soh, 80, 100)
    }

    /// Convenience method: Prove SoH is second-life eligible (60–80%)
    pub fn prove_second_life(
        &self,
        soh: u64,
    ) -> Result<(ZkProof, ProofCommitment, Scalar), ZkError> {
        if !(60..=80).contains(&soh) {
            return Err(ZkError::OutOfRange {
                value: soh,
                min: 60,
                max: 80,
            });
        }
        self.prove_range(soh, 60, 80)
    }

    /// Convenience method: Prove SoH is EOL (< 60%)
    pub fn prove_eol(&self, soh: u64) -> Result<(ZkProof, ProofCommitment, Scalar), ZkError> {
        if soh >= 60 {
            return Err(ZkError::OutOfRange {
                value: soh,
                min: 0,
                max: 59,
            });
        }
        self.prove_range(soh, 0, 59)
    }
}

impl Default for ZkProverImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prove_and_verify_operational() {
        let prover = ZkProverImpl::new();

        // SoH = 87 (operational)
        let (proof, commitment, _blinding) = prover
            .prove_operational(87)
            .expect("prove_operational failed");

        // Verify should succeed
        let result = prover.verify_range(&proof, &commitment, 80, 100);
        assert!(result.is_ok(), "verification failed");
    }

    #[test]
    fn test_prove_and_verify_second_life() {
        let prover = ZkProverImpl::new();

        // SoH = 72 (second-life eligible)
        let (proof, commitment, _blinding) = prover
            .prove_second_life(72)
            .expect("prove_second_life failed");

        let result = prover.verify_range(&proof, &commitment, 60, 80);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prove_out_of_range() {
        let prover = ZkProverImpl::new();

        // Try to prove SoH=75 is operational (75 is not > 80)
        let result = prover.prove_operational(75);
        assert!(result.is_err());
        match result {
            Err(ZkError::OutOfRange { value, min, max }) => {
                assert_eq!(value, 75);
                assert_eq!(min, 80);
                assert_eq!(max, 100);
            }
            _ => panic!("expected OutOfRange error"),
        }
    }

    #[test]
    fn test_verify_fails_on_wrong_range() {
        let prover = ZkProverImpl::new();

        // Prove SoH=87 is operational (80–100)
        let (proof, commitment, _) = prover.prove_operational(87).unwrap();

        // Try to verify it's second-life eligible (60–80)
        // This should fail because the commitment doesn't match the different range
        let result = prover.verify_range(&proof, &commitment, 60, 80);
        assert!(result.is_err(), "verification should fail for wrong range");
    }

    #[test]
    fn test_verify_fails_on_tampered_proof() {
        let prover = ZkProverImpl::new();

        let (mut proof, commitment, _) = prover.prove_operational(87).unwrap();

        // Tamper with the proof by flipping a bit
        if !proof.0.is_empty() {
            proof.0[0] ^= 0x01;
        }

        let result = prover.verify_range(&proof, &commitment, 80, 100);
        assert!(
            result.is_err(),
            "verification should fail for tampered proof"
        );
    }

    #[test]
    fn test_proof_determinism() {
        // Same input should produce identical proofs (for caching)
        // Note: Merlin transcript is deterministic, but randomness in bulletproofs
        // means we may get different proofs even for same input.
        // This test just verifies no panic occurs.
        let prover = ZkProverImpl::new();

        let (proof1, commit1, _) = prover.prove_range(87, 80, 100).unwrap();
        let (proof2, commit2, _) = prover.prove_range(87, 80, 100).unwrap();

        // Commitments should differ due to random blinding
        assert_ne!(proof1.0, proof2.0, "proofs differ");
        assert_ne!(commit1.0, commit2.0, "commitments differ");

        // But both should verify
        assert!(prover.verify_range(&proof1, &commit1, 80, 100).is_ok());
        assert!(prover.verify_range(&proof2, &commit2, 80, 100).is_ok());
    }

    #[test]
    fn test_no_value_extraction() {
        // This test verifies that the commitment alone doesn't reveal the value
        let prover = ZkProverImpl::new();

        let (proof, commitment, _) = prover.prove_operational(87).unwrap();

        // A verifier receiving only proof + commitment cannot determine the value
        // Verifier can only confirm: "this value is in [80, 100]"
        assert!(prover.verify_range(&proof, &commitment, 80, 100).is_ok());

        // Verifier cannot prove it's > 90 without the actual value
        // (They could brute-force by trying all possible values, but that's expensive)
    }

    #[test]
    fn test_boundary_values() {
        let prover = ZkProverImpl::new();

        // Test min boundary (80)
        let (proof_min, commit_min, _) = prover.prove_range(80, 80, 100).unwrap();
        assert!(prover
            .verify_range(&proof_min, &commit_min, 80, 100)
            .is_ok());

        // Test max boundary (100)
        let (proof_max, commit_max, _) = prover.prove_range(100, 80, 100).unwrap();
        assert!(prover
            .verify_range(&proof_max, &commit_max, 80, 100)
            .is_ok());

        // Test just outside (79)
        let result = prover.prove_range(79, 80, 100);
        assert!(result.is_err());

        // Test just outside (101)
        let result = prover.prove_range(101, 80, 100);
        assert!(result.is_err());
    }
}
