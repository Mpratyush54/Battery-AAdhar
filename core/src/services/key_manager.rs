//! key_manager.rs — HKDF-based 3-tier key hierarchy stub
//!
//! Key hierarchy (matches dbschma.txt tables):
//!   root_keys  →  kek_keys  →  battery_keys (DEK per BPAN)
//!
//! This is a stub — concrete HKDF derivation wires in on Day 10.
//! All types are defined here so the rest of the codebase can compile
//! against the interface from Day 1.

use zeroize::Zeroize;

/// Key status values — mirror `key_status` / `status` DB columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStatus {
    Active,
    Retired,
    Destroyed,
}

impl KeyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyStatus::Active    => "active",
            KeyStatus::Retired   => "retired",
            KeyStatus::Destroyed => "destroyed",
        }
    }
}

/// A 32-byte raw key — zeroized on drop.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct RawKey(pub [u8; 32]);

impl std::fmt::Debug for RawKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material
        write!(f, "RawKey([REDACTED])")
    }
}

/// Reference to a KEK version — used when wrapping/unwrapping DEKs.
#[derive(Debug, Clone)]
pub struct KekRef {
    pub id:      uuid::Uuid,
    pub version: i32,
}

/// A wrapped (encrypted) data-encryption key for a single BPAN.
#[derive(Debug, Clone)]
pub struct WrappedDek {
    pub bpan:             String,
    pub encrypted_dek:    Vec<u8>,
    pub kek_version:      i32,
    pub cipher_algorithm: String,
    pub cipher_version:   i32,
}

/// Errors from key management operations.
#[derive(Debug)]
pub enum KeyManagerError {
    /// Root key is not loaded or not hardware-backed.
    RootKeyUnavailable,
    /// KEK for the given version does not exist.
    KekNotFound { version: i32 },
    /// DEK for the given BPAN does not exist.
    DekNotFound { bpan: String },
    /// Key derivation via HKDF failed.
    DerivationFailed(String),
    /// Key wrapping or unwrapping failed.
    WrappingFailed(String),
    /// DB operation failed.
    StorageError(String),
}

impl std::fmt::Display for KeyManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyManagerError::RootKeyUnavailable         => write!(f, "root key unavailable"),
            KeyManagerError::KekNotFound { version }    => write!(f, "KEK version {version} not found"),
            KeyManagerError::DekNotFound { bpan }       => write!(f, "DEK for BPAN {bpan} not found"),
            KeyManagerError::DerivationFailed(msg)      => write!(f, "HKDF derivation failed: {msg}"),
            KeyManagerError::WrappingFailed(msg)        => write!(f, "key wrapping failed: {msg}"),
            KeyManagerError::StorageError(msg)          => write!(f, "storage error: {msg}"),
        }
    }
}

impl std::error::Error for KeyManagerError {}

/// The primary interface for the 3-tier key hierarchy.
///
/// # Tier summary
/// ```text
/// Root key (hardware-backed, env or vault)
///   └── KEK  (AES-256-GCM, stored encrypted in kek_keys)
///         └── DEK  (AES-256-GCM, per BPAN, stored in battery_keys)
/// ```
///
/// Callers (encryption service, signing service) only interact with
/// `get_dek_for_bpan` and `create_dek_for_bpan`. The root and KEK
/// tiers are internal to this service.
pub trait KeyManager: Send + Sync {
    /// Derive or retrieve the current KEK.
    /// Only called internally by DEK operations.
    fn get_current_kek(&self) -> Result<(RawKey, KekRef), KeyManagerError>;

    /// Create a new DEK for `bpan`, wrap it with the current KEK,
    /// and persist it to `battery_keys`.
    fn create_dek_for_bpan(
        &self,
        bpan: &str,
    ) -> Result<WrappedDek, KeyManagerError>;

    /// Retrieve and unwrap the DEK for `bpan`.
    /// Returns the plaintext DEK for use in encryption/decryption.
    /// The returned `RawKey` is zeroized when dropped.
    fn get_dek_for_bpan(
        &self,
        bpan: &str,
    ) -> Result<RawKey, KeyManagerError>;

    /// Rotate the DEK for `bpan`: generate new DEK, re-encrypt all
    /// existing encrypted fields, persist new DEK version.
    /// Logs to `key_rotation_log`.
    fn rotate_dek(
        &self,
        bpan:         &str,
        rotated_by:   uuid::Uuid,
    ) -> Result<(), KeyManagerError>;

    /// Destroy the DEK for `bpan` (EOL battery). After this call,
    /// private fields for this BPAN become permanently unreadable.
    /// Logs to `key_destruction_log`.
    fn destroy_dek(
        &self,
        bpan:              &str,
        destroyed_by:      uuid::Uuid,
        destruction_method: &str,
    ) -> Result<(), KeyManagerError>;
}
