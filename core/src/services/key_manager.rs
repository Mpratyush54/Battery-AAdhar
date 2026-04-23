//! key_manager.rs — 3-tier key hierarchy: root → KEK → DEK
//!
//! HKDF-SHA256 is used for all derivations.
//! All key material is wrapped in Zeroize types to prevent leaks.

use crate::models::*;
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroize;
use std::fmt;
use uuid::Uuid;
use chrono::Utc;

/// Raw 32-byte key material (automatically zeroized on drop)
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct RawKey {
    bytes: [u8; 32],
}

impl fmt::Debug for RawKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawKey([REDACTED])")
    }
}

impl RawKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        RawKey { bytes }
    }

    pub fn from_vec(vec: Vec<u8>) -> Result<Self, KeyManagerError> {
        if vec.len() != 32 {
            return Err(KeyManagerError::DerivationFailed(
                format!("expected 32 bytes, got {}", vec.len()),
            ));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&vec);
        Ok(RawKey { bytes })
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }
}

/// Reference to a KEK version for lookups
#[derive(Debug, Clone)]
pub struct KekRef {
    pub id: Uuid,
    pub version: i32,
}

/// Wrapped (encrypted) DEK for a single BPAN
#[derive(Debug, Clone)]
pub struct WrappedDek {
    pub bpan: String,
    pub encrypted_dek: Vec<u8>,
    pub kek_version: i32,
    pub cipher_algorithm: String,
    pub cipher_version: i32,
}

/// Key manager errors
#[derive(Debug)]
pub enum KeyManagerError {
    RootKeyUnavailable,
    KekNotFound { version: i32 },
    DekNotFound { bpan: String },
    DerivationFailed(String),
    WrappingFailed(String),
    StorageError(String),
    InvalidKeyMaterial(String),
}

impl fmt::Display for KeyManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyManagerError::RootKeyUnavailable => write!(f, "root key unavailable"),
            KeyManagerError::KekNotFound { version } => write!(f, "KEK version {} not found", version),
            KeyManagerError::DekNotFound { bpan } => write!(f, "DEK for BPAN {} not found", bpan),
            KeyManagerError::DerivationFailed(msg) => write!(f, "HKDF derivation failed: {}", msg),
            KeyManagerError::WrappingFailed(msg) => write!(f, "key wrapping failed: {}", msg),
            KeyManagerError::StorageError(msg) => write!(f, "storage error: {}", msg),
            KeyManagerError::InvalidKeyMaterial(msg) => write!(f, "invalid key material: {}", msg),
        }
    }
}

impl std::error::Error for KeyManagerError {}

/// The concrete KeyManager implementation.
/// Handles all 3-tier key operations.
pub struct KeyManagerImpl {
    root_key: RawKey,
    // In production, add a DB connection pool here for persistence
    // For Day 3, we store in memory as a proof-of-concept
}

impl KeyManagerImpl {
    /// Create a new KeyManager with a root key.
    /// The root key is typically loaded from an environment variable or HSM.
    pub fn new(root_key_bytes: &[u8; 32]) -> Result<Self, KeyManagerError> {
        if root_key_bytes.iter().all(|&b| b == 0) {
            return Err(KeyManagerError::RootKeyUnavailable);
        }
        Ok(KeyManagerImpl {
            root_key: RawKey::new(*root_key_bytes),
        })
    }

    /// Generate a new root key (256-bit random).
    /// Returns the key and metadata for storage in `root_keys` table.
    pub fn generate_root_key() -> Result<(RawKey, RootKeyMetadata), KeyManagerError> {
        let mut bytes = [0u8; 32];
        // Use a proper CSPRNG in production (e.g., getrandom crate)
        // For testing, this is a placeholder
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut bytes);

        let metadata = RootKeyMetadata {
            id: Uuid::new_v4(),
            hardware_backed: false, // Set to true if using HSM
            status: "active".to_string(),
            created_at: Utc::now(),
            retired_at: None,
        };

        Ok((RawKey::new(bytes), metadata))
    }

    /// Derive a KEK from the root key using HKDF-SHA256.
    /// Includes version number and timestamp in the derivation.
    /// Returns the plaintext KEK (caller must wrap it with root key before storage).
    pub fn derive_kek(&self, version: i32) -> Result<(RawKey, KekMetadata), KeyManagerError> {
        let hkdf = Hkdf::<Sha256>::new(None, self.root_key.as_bytes());

        // Info: include version number so each KEK is unique
        let info = format!("BPA-KEK-v{}", version).into_bytes();

        let mut kek_bytes = [0u8; 32];
        hkdf.expand(&info, &mut kek_bytes)
            .map_err(|e| KeyManagerError::DerivationFailed(format!("HKDF expand: {}", e)))?;

        let metadata = KekMetadata {
            id: Uuid::new_v4(),
            version,
            root_key_id: Uuid::nil(), // Will be set by caller
            status: "active".to_string(),
            created_at: Utc::now(),
            retired_at: None,
        };

        Ok((RawKey::new(kek_bytes), metadata))
    }

    /// Derive a DEK for a specific BPAN using HKDF-SHA256 with the KEK.
    /// Each DEK is unique per BPAN due to HKDF's expansion using BPAN as info.
    pub fn derive_dek(
        &self,
        kek: &RawKey,
        bpan: &str,
    ) -> Result<RawKey, KeyManagerError> {
        let hkdf = Hkdf::<Sha256>::new(None, kek.as_bytes());

        // Info: include BPAN so each DEK is unique
        let info = format!("BPA-DEK-{}", bpan).into_bytes();

        let mut dek_bytes = [0u8; 32];
        hkdf.expand(&info, &mut dek_bytes)
            .map_err(|e| KeyManagerError::DerivationFailed(format!("HKDF expand: {}", e)))?;

        Ok(RawKey::new(dek_bytes))
    }

    /// Wrap a DEK using AES-256-GCM with the KEK.
    /// Returns ciphertext + nonce + tag suitable for storage in `battery_keys`.
    pub fn wrap_dek(
        &self,
        kek: &RawKey,
        dek: &RawKey,
        bpan: &str,
    ) -> Result<Vec<u8>, KeyManagerError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use rand::RngCore;

        // Generate a random 96-bit nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(kek.as_bytes())
            .map_err(|_| KeyManagerError::WrappingFailed("invalid KEK length".to_string()))?;

        // AAD: BPAN (prevents DEK reuse across different batteries)
        let aad = bpan.as_bytes();

        let mut ciphertext = cipher
            .encrypt(nonce, aes_gcm::aead::Payload { msg: dek.as_bytes(), aad })
            .map_err(|e| KeyManagerError::WrappingFailed(e.to_string()))?;

        // Prepend nonce to ciphertext for storage (nonce doesn't need to be secret)
        let mut wrapped = nonce_bytes.to_vec();
        wrapped.append(&mut ciphertext);

        Ok(wrapped)
    }

    /// Unwrap a DEK using AES-256-GCM with the KEK.
    /// Extracts nonce from the wrapped ciphertext.
    pub fn unwrap_dek(
        &self,
        kek: &RawKey,
        wrapped_dek: &[u8],
        bpan: &str,
    ) -> Result<RawKey, KeyManagerError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        if wrapped_dek.len() < 12 {
            return Err(KeyManagerError::WrappingFailed(
                "wrapped DEK too short (no nonce)".to_string(),
            ));
        }

        let nonce = Nonce::from_slice(&wrapped_dek[..12]);
        let ciphertext = &wrapped_dek[12..];

        let cipher = Aes256Gcm::new_from_slice(kek.as_bytes())
            .map_err(|_| KeyManagerError::WrappingFailed("invalid KEK length".to_string()))?;

        let aad = bpan.as_bytes();

        let dek_bytes = cipher
            .decrypt(nonce, aes_gcm::aead::Payload { msg: ciphertext, aad })
            .map_err(|e| KeyManagerError::WrappingFailed(e.to_string()))?;

        RawKey::from_vec(dek_bytes)
    }

    /// Full end-to-end: derive KEK, then DEK for BPAN, wrap DEK, return wrapped version.
    /// Used during battery registration to set up encryption for the first time.
    pub fn create_dek_for_bpan(
        &self,
        bpan: &str,
        kek_version: i32,
    ) -> Result<WrappedDek, KeyManagerError> {
        // Step 1: Derive KEK
        let (kek, _) = self.derive_kek(kek_version)?;

        // Step 2: Derive DEK
        let dek = self.derive_dek(&kek, bpan)?;

        // Step 3: Wrap DEK
        let wrapped = self.wrap_dek(&kek, &dek, bpan)?;

        Ok(WrappedDek {
            bpan: bpan.to_string(),
            encrypted_dek: wrapped,
            kek_version,
            cipher_algorithm: "AES-256-GCM".to_string(),
            cipher_version: 1,
        })
    }

    /// Get the plaintext DEK for a BPAN (requires wrapped DEK from storage).
    /// Used during encryption/decryption operations.
    pub fn get_dek_for_bpan(
        &self,
        bpan: &str,
        wrapped_dek: &[u8],
        kek_version: i32,
    ) -> Result<RawKey, KeyManagerError> {
        // Step 1: Derive KEK (must be same version as when DEK was wrapped)
        let (kek, _) = self.derive_kek(kek_version)?;

        // Step 2: Unwrap DEK
        self.unwrap_dek(&kek, wrapped_dek, bpan)
    }
}

// Metadata structs for storage (will be persisted in Day 7)
#[derive(Debug, Clone)]
pub struct RootKeyMetadata {
    pub id: Uuid,
    pub hardware_backed: bool,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub retired_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct KekMetadata {
    pub id: Uuid,
    pub version: i32,
    pub root_key_id: Uuid,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub retired_at: Option<chrono::DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_root_key() {
        let (key, metadata) = KeyManagerImpl::generate_root_key().unwrap();
        assert_eq!(key.as_bytes().len(), 32);
        assert_eq!(metadata.status, "active");
    }

    #[test]
    fn test_derive_kek_deterministic() {
        let root_bytes = [1u8; 32];
        let km = KeyManagerImpl::new(&root_bytes).unwrap();

        let (kek1, _) = km.derive_kek(1).unwrap();
        let (kek2, _) = km.derive_kek(1).unwrap();

        // Same inputs → same KEK
        assert_eq!(kek1.as_bytes(), kek2.as_bytes());
    }

    #[test]
    fn test_derive_kek_version_isolation() {
        let root_bytes = [1u8; 32];
        let km = KeyManagerImpl::new(&root_bytes).unwrap();

        let (kek1, _) = km.derive_kek(1).unwrap();
        let (kek2, _) = km.derive_kek(2).unwrap();

        // Different versions → different KEKs
        assert_ne!(kek1.as_bytes(), kek2.as_bytes());
    }

    #[test]
    fn test_derive_dek_unique_per_bpan() {
        let root_bytes = [1u8; 32];
        let km = KeyManagerImpl::new(&root_bytes).unwrap();

        let (kek, _) = km.derive_kek(1).unwrap();
        let dek1 = km.derive_dek(&kek, "MY008A6FKKKLC1DH80001").unwrap();
        let dek2 = km.derive_dek(&kek, "MY008A6FKKKLC1DH80002").unwrap();

        // Different BPANs → different DEKs
        assert_ne!(dek1.as_bytes(), dek2.as_bytes());
    }

    #[test]
    fn test_wrap_unwrap_roundtrip() {
        let root_bytes = [1u8; 32];
        let km = KeyManagerImpl::new(&root_bytes).unwrap();

        let bpan = "MY008A6FKKKLC1DH80001";
        let (kek, _) = km.derive_kek(1).unwrap();
        let dek_original = km.derive_dek(&kek, bpan).unwrap();

        // Wrap
        let wrapped = km.wrap_dek(&kek, &dek_original, bpan).unwrap();

        // Unwrap
        let dek_recovered = km.unwrap_dek(&kek, &wrapped, bpan).unwrap();

        // Must be identical
        assert_eq!(dek_original.as_bytes(), dek_recovered.as_bytes());
    }

    #[test]
    fn test_wrap_unwrap_aad_mismatch() {
        let root_bytes = [1u8; 32];
        let km = KeyManagerImpl::new(&root_bytes).unwrap();

        let bpan = "MY008A6FKKKLC1DH80001";
        let (kek, _) = km.derive_kek(1).unwrap();
        let dek = km.derive_dek(&kek, bpan).unwrap();

        // Wrap with one BPAN as AAD
        let wrapped = km.wrap_dek(&kek, &dek, bpan).unwrap();

        // Try to unwrap with different BPAN as AAD (should fail)
        let result = km.unwrap_dek(&kek, &wrapped, "MY008A6FKKKLC1DH80999");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_dek_for_bpan_endtoend() {
        let root_bytes = [1u8; 32];
        let km = KeyManagerImpl::new(&root_bytes).unwrap();

        let bpan = "MY008A6FKKKLC1DH80001";
        let wrapped_dek = km.create_dek_for_bpan(bpan, 1).unwrap();

        assert_eq!(wrapped_dek.bpan, bpan);
        assert_eq!(wrapped_dek.kek_version, 1);
        assert_eq!(wrapped_dek.cipher_algorithm, "AES-256-GCM");
        assert!(wrapped_dek.encrypted_dek.len() > 32); // Wrapped = nonce + ciphertext
    }

    #[test]
    fn test_get_dek_for_bpan() {
        let root_bytes = [1u8; 32];
        let km = KeyManagerImpl::new(&root_bytes).unwrap();

        let bpan = "MY008A6FKKKLC1DH80001";

        // Create DEK
        let wrapped_dek_obj = km.create_dek_for_bpan(bpan, 1).unwrap();

        // Retrieve DEK (would normally come from DB)
        let retrieved_dek = km
            .get_dek_for_bpan(bpan, &wrapped_dek_obj.encrypted_dek, 1)
            .unwrap();

        // Should be usable for encryption
        assert_eq!(retrieved_dek.as_bytes().len(), 32);
    }
}
