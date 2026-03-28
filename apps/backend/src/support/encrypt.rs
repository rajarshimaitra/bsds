//! AES-256-GCM field-level encryption for PII data at rest.
//!
//! Format: `enc:<base64(iv + ciphertext + auth_tag)>`
//!   - IV: 12 bytes (96 bits), randomly generated per encryption
//!   - Auth tag: 16 bytes (128 bits)
//!   - Prefix `enc:` marks encrypted values for idempotency detection
//!
//! Key: 32-byte key passed as a `&[u8; 32]` slice. Callers load this from
//! the `ENCRYPTION_KEY` env var (64 hex chars) at startup.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, AeadCore, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Prefix marking an already-encrypted value. Prevents double-encryption.
const ENC_PREFIX: &str = "enc:";

/// GCM IV length in bytes. 12 bytes (96 bits) is the recommended GCM IV size.
const IV_LENGTH: usize = 12;

/// GCM auth tag length in bytes. 16 bytes = 128-bit tag (maximum).
const AUTH_TAG_LENGTH: usize = 16;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum EncryptError {
    #[error("Value is not encrypted (missing \"enc:\" prefix)")]
    NotEncrypted,

    #[error("Encrypted blob is too short to be valid (need at least {min} bytes, got {got})")]
    BlobTooShort { min: usize, got: usize },

    #[error("Base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("AES-GCM encryption failed")]
    EncryptionFailed,

    #[error("AES-GCM decryption failed (wrong key or tampered data)")]
    DecryptionFailed,

    #[error("Decrypted bytes are not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

// ---------------------------------------------------------------------------
// Core encrypt / decrypt
// ---------------------------------------------------------------------------

/// Encrypt a plaintext string using AES-256-GCM.
///
/// A fresh random 12-byte IV is generated for every call so that two
/// encryptions of the same plaintext produce different ciphertexts.
///
/// Returns `enc:<base64>` where the base64 payload is `[IV | ciphertext | auth_tag]`.
pub fn encrypt(plaintext: &str, key: &[u8]) -> Result<String, EncryptError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| EncryptError::EncryptionFailed)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // aes-gcm appends the 16-byte auth tag to the ciphertext automatically.
    let ciphertext_with_tag = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| EncryptError::EncryptionFailed)?;

    // Layout must match the TypeScript version: [12-byte IV] [N-byte ciphertext] [16-byte tag]
    // aes-gcm already produces [ciphertext | tag], so we prepend the nonce.
    let mut blob = Vec::with_capacity(IV_LENGTH + ciphertext_with_tag.len());
    blob.extend_from_slice(nonce.as_slice());
    blob.extend_from_slice(&ciphertext_with_tag);

    let encoded = BASE64.encode(&blob);
    Ok(format!("{ENC_PREFIX}{encoded}"))
}

/// Decrypt a value produced by [`encrypt`].
///
/// Expects the `enc:<base64>` format. Returns the original UTF-8 plaintext.
pub fn decrypt(ciphertext: &str, key: &[u8]) -> Result<String, EncryptError> {
    if !is_encrypted(ciphertext) {
        return Err(EncryptError::NotEncrypted);
    }

    let encoded = &ciphertext[ENC_PREFIX.len()..];
    let blob = BASE64.decode(encoded)?;

    let min_len = IV_LENGTH + AUTH_TAG_LENGTH;
    if blob.len() < min_len {
        return Err(EncryptError::BlobTooShort {
            min: min_len,
            got: blob.len(),
        });
    }

    let iv = Nonce::from_slice(&blob[..IV_LENGTH]);
    // Everything after IV is ciphertext + auth_tag, which aes-gcm expects together.
    let ciphertext_with_tag = &blob[IV_LENGTH..];

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| EncryptError::DecryptionFailed)?;
    let plaintext_bytes = cipher
        .decrypt(iv, ciphertext_with_tag)
        .map_err(|_| EncryptError::DecryptionFailed)?;

    Ok(String::from_utf8(plaintext_bytes)?)
}

// ---------------------------------------------------------------------------
// Heuristic detection
// ---------------------------------------------------------------------------

/// Returns `true` if the value looks like an output of [`encrypt`].
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENC_PREFIX)
}

// ---------------------------------------------------------------------------
// Null-safe wrappers
// ---------------------------------------------------------------------------

/// Encrypt a value if it is not already encrypted and not `None`.
pub fn encrypt_if_needed(value: Option<&str>, key: &[u8]) -> Result<Option<String>, EncryptError> {
    match value {
        None => Ok(None),
        Some(v) if is_encrypted(v) => Ok(Some(v.to_string())),
        Some(v) => encrypt(v, key).map(Some),
    }
}

/// Decrypt a value if it is encrypted, otherwise return it unchanged.
/// Returns `None` unchanged. Never panics on plaintext inputs.
pub fn decrypt_if_needed(value: Option<&str>, key: &[u8]) -> Option<String> {
    match value {
        None => None,
        Some(v) if !is_encrypted(v) => Some(v.to_string()),
        Some(v) => match decrypt(v, key) {
            Ok(plaintext) => Some(plaintext),
            Err(err) => {
                tracing::error!("[encrypt] Failed to decrypt field value: {err}");
                Some(v.to_string())
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [0xABu8; 32]
    }

    #[test]
    fn round_trip() {
        let key = test_key();
        let plaintext = "hello world";
        let encrypted = encrypt(plaintext, &key).unwrap();
        assert!(is_encrypted(&encrypted));
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn different_ciphertexts_for_same_plaintext() {
        let key = test_key();
        let a = encrypt("test", &key).unwrap();
        let b = encrypt("test", &key).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn decrypt_not_encrypted_returns_error() {
        let key = test_key();
        assert!(matches!(decrypt("plain", &key), Err(EncryptError::NotEncrypted)));
    }
}
