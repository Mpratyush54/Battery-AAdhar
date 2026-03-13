use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use tracing::{debug, instrument};

use crate::errors::{BpaError, BpaResult};

/// Handles all AES-256-GCM encryption and decryption for the BPA platform.
/// Extracted from main.rs so every service can use it without coupling to the controller.
#[derive(Clone)]
pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    /// Create a new EncryptionService from a 32-byte key string.
    pub fn new(master_key: &str) -> BpaResult<Self> {
        if master_key.len() != 32 {
            return Err(BpaError::Encryption(
                "ENCRYPTION_KEY must be exactly 32 bytes".into(),
            ));
        }
        let aes_key = aes_gcm::Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
        let cipher = Aes256Gcm::new(aes_key);
        Ok(Self { cipher })
    }

    /// Encrypt a plaintext string into a base64-encoded ciphertext (IV prepended).
    #[instrument(name = "encrypt_buffer", skip(self, plaintext))]
    pub fn encrypt(&self, plaintext: &str) -> BpaResult<String> {
        let mut iv_buffer = [0u8; 12];
        OsRng.fill_bytes(&mut iv_buffer);
        let iv = Nonce::from_slice(&iv_buffer);

        let ciphertext_buffer = self
            .cipher
            .encrypt(iv, plaintext.as_bytes())
            .map_err(|e| BpaError::Encryption(format!("Encryption failed: {}", e)))?;

        let mut combined = iv_buffer.to_vec();
        combined.extend(ciphertext_buffer);

        debug!("Encrypted {} bytes of plaintext", plaintext.len());
        Ok(general_purpose::STANDARD.encode(&combined))
    }

    /// Decrypt a base64-encoded ciphertext back to plaintext.
    #[instrument(name = "decrypt_string", skip(self, base64_cipher))]
    pub fn decrypt(&self, base64_cipher: &str) -> BpaResult<String> {
        let combined = general_purpose::STANDARD
            .decode(base64_cipher)
            .map_err(|e| BpaError::Encryption(format!("Invalid base64: {}", e)))?;

        if combined.len() < 12 {
            return Err(BpaError::Encryption("Invalid cipher length".into()));
        }

        let iv = Nonce::from_slice(&combined[..12]);
        let ciphertext = &combined[12..];

        let plaintext_buffer = self
            .cipher
            .decrypt(iv, ciphertext)
            .map_err(|e| BpaError::Encryption(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext_buffer)
            .map_err(|e| BpaError::Encryption(format!("Invalid utf8: {}", e)))
    }

    /// Encrypt arbitrary bytes (for binary payloads like telemetry).
    pub fn encrypt_bytes(&self, data: &[u8]) -> BpaResult<String> {
        let mut iv_buffer = [0u8; 12];
        OsRng.fill_bytes(&mut iv_buffer);
        let iv = Nonce::from_slice(&iv_buffer);

        let ciphertext = self
            .cipher
            .encrypt(iv, data)
            .map_err(|e| BpaError::Encryption(format!("Encryption failed: {}", e)))?;

        let mut combined = iv_buffer.to_vec();
        combined.extend(ciphertext);

        Ok(general_purpose::STANDARD.encode(&combined))
    }

    /// Decrypt base64-encoded ciphertext back to raw bytes.
    pub fn decrypt_bytes(&self, base64_cipher: &str) -> BpaResult<Vec<u8>> {
        let combined = general_purpose::STANDARD
            .decode(base64_cipher)
            .map_err(|e| BpaError::Encryption(format!("Invalid base64: {}", e)))?;

        if combined.len() < 12 {
            return Err(BpaError::Encryption("Invalid cipher length".into()));
        }

        let iv = Nonce::from_slice(&combined[..12]);
        let ciphertext = &combined[12..];

        self.cipher
            .decrypt(iv, ciphertext)
            .map_err(|e| BpaError::Encryption(format!("Decryption failed: {}", e)))
    }
}
