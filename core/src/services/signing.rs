//! signing.rs — Ed25519 signing service implementation
//!
//! Each manufacturer has a unique keypair. When a battery is registered,
//! the static data is signed with the manufacturer's private key.
//! This signature is stored and can be verified by any stakeholder later.

use chrono::Utc;
use ed25519_dalek::{SignatureError, SigningKey, VerifyingKey};
use std::fmt;
use uuid::Uuid;
use zeroize::Zeroize;

/// A 32-byte Ed25519 signing key (private key seed)
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct PrivateKeySeed([u8; 32]);

impl fmt::Debug for PrivateKeySeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrivateKeySeed([REDACTED])")
    }
}

impl PrivateKeySeed {
    pub fn new(bytes: [u8; 32]) -> Self {
        PrivateKeySeed(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A 32-byte Ed25519 public key
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let bytes = hex::decode(hex_str).map_err(|e| format!("hex decode failed: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!("expected 32 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(PublicKey(arr))
    }
}

/// A 64-byte Ed25519 signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureWrap([u8; 64]);

impl SignatureWrap {
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        SignatureWrap(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let bytes = hex::decode(hex_str).map_err(|e| format!("hex decode failed: {}", e))?;
        if bytes.len() != 64 {
            return Err(format!("expected 64 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(SignatureWrap(arr))
    }
}

/// Error type for signing operations
#[derive(Debug)]
pub enum SigningError {
    KeyError(String),
    SigningFailed(String),
    VerificationFailed,
    MalformedKey,
}

impl fmt::Display for SigningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SigningError::KeyError(msg) => write!(f, "key error: {}", msg),
            SigningError::SigningFailed(msg) => write!(f, "signing failed: {}", msg),
            SigningError::VerificationFailed => write!(f, "signature verification failed"),
            SigningError::MalformedKey => write!(f, "malformed key material"),
        }
    }
}

impl std::error::Error for SigningError {}

impl From<SignatureError> for SigningError {
    fn from(err: SignatureError) -> Self {
        SigningError::SigningFailed(err.to_string())
    }
}

/// Keypair metadata for storage in `certificates` table
#[derive(Debug, Clone)]
pub struct KeypairMetadata {
    pub id: Uuid,
    pub manufacturer_id: String,
    pub public_key: PublicKey,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub retired_at: Option<chrono::DateTime<Utc>>,
}

/// The concrete SigningService implementation
pub struct SigningServiceImpl {
    // In production, this would cache the active keypairs
    // For Day 4, we'll retrieve them from DB via a repository
}

impl Default for SigningServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl SigningServiceImpl {
    pub fn new() -> Self {
        SigningServiceImpl {}
    }

    /// Generate a new Ed25519 keypair for a manufacturer.
    /// The private key is NOT returned to the caller.
    /// Returns the public key + a key_id for future reference.
    pub fn generate_keypair() -> Result<(PrivateKeySeed, PublicKey), SigningError> {
        // Generate random 32-byte seed
        let mut seed = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut seed);

        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        let public_key = PublicKey::from_bytes(*verifying_key.as_bytes());

        Ok((PrivateKeySeed::new(seed), public_key))
    }

    /// Sign a BPAN + metadata blob with a manufacturer's private key.
    /// The message includes the BPAN + static battery data.
    /// Returns a 64-byte Ed25519 signature.
    pub fn sign_message(
        private_key_seed: &PrivateKeySeed,
        message: &[u8],
    ) -> Result<SignatureWrap, SigningError> {
        let signing_key = SigningKey::from_bytes(private_key_seed.as_bytes());
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(message);

        Ok(SignatureWrap::from_bytes(signature.to_bytes()))
    }

    /// Verify a signature over a message using a public key.
    /// Used by any stakeholder to verify battery data hasn't been tampered.
    pub fn verify_signature(
        public_key: &PublicKey,
        message: &[u8],
        signature: &SignatureWrap,
    ) -> Result<(), SigningError> {
        // Convert public key to ed25519_dalek::VerifyingKey
        let verifying_key = VerifyingKey::from_bytes(public_key.as_bytes())
            .map_err(|_| SigningError::MalformedKey)?;

        // Convert signature to ed25519_dalek::Signature
        let sig = ed25519_dalek::Signature::from_bytes(signature.as_bytes());

        use ed25519_dalek::Verifier;
        verifying_key.verify(message, &sig)?;

        Ok(())
    }

    /// Sign a complete battery record (BPAN + static data).
    /// This is called during battery registration.
    pub fn sign_battery_record(
        private_key_seed: &PrivateKeySeed,
        bpan: &str,
        static_data: &str, // JSON-serialized battery static data
    ) -> Result<SignatureWrap, SigningError> {
        // Construct the message: BPAN || static_data
        let mut message = Vec::new();
        message.extend_from_slice(bpan.as_bytes());
        message.extend_from_slice(b"||");
        message.extend_from_slice(static_data.as_bytes());

        Self::sign_message(private_key_seed, &message)
    }
}

// Export types for gRPC
pub use ed25519_dalek::Signature as DalekSignature;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let (seed, pubkey) = SigningServiceImpl::generate_keypair().unwrap();
        assert_eq!(seed.as_bytes().len(), 32);
        assert_eq!(pubkey.as_bytes().len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let (seed, pubkey) = SigningServiceImpl::generate_keypair().unwrap();
        let message = b"test message";

        let signature = SigningServiceImpl::sign_message(&seed, message).unwrap();

        // Verify should succeed
        let result = SigningServiceImpl::verify_signature(&pubkey, message, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_tampered_message() {
        let (seed, pubkey) = SigningServiceImpl::generate_keypair().unwrap();
        let message = b"test message";
        let tampered = b"tampered message";

        let signature = SigningServiceImpl::sign_message(&seed, message).unwrap();

        // Verify should fail because message was tampered
        let result = SigningServiceImpl::verify_signature(&pubkey, tampered, &signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_wrong_key() {
        let (seed1, _pubkey1) = SigningServiceImpl::generate_keypair().unwrap();
        let (_seed2, pubkey2) = SigningServiceImpl::generate_keypair().unwrap();
        let message = b"test message";

        let signature = SigningServiceImpl::sign_message(&seed1, message).unwrap();

        // Verify with wrong public key should fail
        let result = SigningServiceImpl::verify_signature(&pubkey2, message, &signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_battery_record() {
        let (seed, pubkey) = SigningServiceImpl::generate_keypair().unwrap();
        let bpan = "MY008A6FKKKLC1DH80001";
        let static_data = r#"{"capacity_kwh":30,"chemistry":"NMC"}"#;

        let signature = SigningServiceImpl::sign_battery_record(&seed, bpan, static_data).unwrap();

        // Construct the message the same way for verification
        let mut message = Vec::new();
        message.extend_from_slice(bpan.as_bytes());
        message.extend_from_slice(b"||");
        message.extend_from_slice(static_data.as_bytes());

        // Verify
        let result = SigningServiceImpl::verify_signature(&pubkey, &message, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_public_key_hex_roundtrip() {
        let (_seed, pubkey) = SigningServiceImpl::generate_keypair().unwrap();

        let hex = pubkey.to_hex();
        let recovered = PublicKey::from_hex(&hex).unwrap();

        assert_eq!(pubkey, recovered);
    }

    #[test]
    fn test_signature_hex_roundtrip() {
        let (seed, _pubkey) = SigningServiceImpl::generate_keypair().unwrap();
        let message = b"test";

        let signature = SigningServiceImpl::sign_message(&seed, message).unwrap();
        let hex = signature.to_hex();
        let recovered = SignatureWrap::from_hex(&hex).unwrap();

        assert_eq!(signature, recovered);
    }
}
