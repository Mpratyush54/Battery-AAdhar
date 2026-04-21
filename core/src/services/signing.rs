//! signing.rs — Ed25519 signing and verification service trait
//!
//! Concrete implementation wires in `ed25519-dalek` on Day 13.
//! All key material is wrapped in `Zeroize` types so it is cleared
//! from memory when dropped.

use zeroize::Zeroize;

/// A 64-byte Ed25519 signature.
#[derive(Debug, Clone)]
pub struct Signature(pub [u8; 64]);

/// A 32-byte Ed25519 public key.
#[derive(Debug, Clone)]
pub struct PublicKey(pub [u8; 32]);

/// A 32-byte Ed25519 private key seed — zeroized on drop.
#[derive(Debug, Clone, Zeroize)]
#[zeroize(drop)]
pub struct PrivateKeySeed(pub [u8; 32]);

/// Error type for signing operations.
#[derive(Debug)]
pub enum SigningError {
    /// Key generation or derivation failed.
    KeyError(String),
    /// Signing operation failed.
    SigningFailed(String),
    /// The signature did not verify against the provided public key and message.
    InvalidSignature,
    /// The provided key material is malformed or has an invalid length.
    MalformedKey,
}

impl std::fmt::Display for SigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SigningError::KeyError(msg)      => write!(f, "key error: {msg}"),
            SigningError::SigningFailed(msg) => write!(f, "signing failed: {msg}"),
            SigningError::InvalidSignature   => write!(f, "signature verification failed"),
            SigningError::MalformedKey       => write!(f, "malformed key material"),
        }
    }
}

impl std::error::Error for SigningError {}

/// The primary interface for Ed25519 signing operations.
///
/// # Key lifecycle
/// - Keys are derived per-manufacturer using HKDF (see `key_manager.rs`).
/// - Public keys are stored in the `certificates` table.
/// - Private key seeds never leave this service boundary.
pub trait SigningService: Send + Sync {
    /// Sign `message` with the manufacturer's private key identified by
    /// `manufacturer_id`. The signing service retrieves the key from the
    /// key manager internally — callers never handle private key material.
    fn sign(
        &self,
        manufacturer_id: &str,
        message:          &[u8],
    ) -> Result<Signature, SigningError>;

    /// Verify `signature` over `message` using `public_key`.
    ///
    /// This is a pure verification — no key lookup required.
    fn verify(
        &self,
        public_key: &PublicKey,
        message:     &[u8],
        signature:   &Signature,
    ) -> Result<(), SigningError>;

    /// Generate a new keypair for a manufacturer.
    /// Returns `(public_key, key_id)` — the private seed is stored internally
    /// and the key_id is what callers use for future `sign()` calls.
    fn generate_keypair(
        &self,
        manufacturer_id: &str,
    ) -> Result<(PublicKey, String), SigningError>;

    /// Retrieve the public key for a given `key_id`.
    fn get_public_key(
        &self,
        key_id: &str,
    ) -> Result<PublicKey, SigningError>;
}
