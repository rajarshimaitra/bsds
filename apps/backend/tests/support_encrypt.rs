//! # Encryption Support Tests
//!
//! Covers: encrypt/decrypt round-trip, idempotency (double-encrypt returns
//!         same enc: value), two encryptions of same plaintext produce different
//!         ciphertexts (IV randomisation), decrypt of non-encrypted value returns
//!         error, blob-too-short error, wrong-key authentication failure,
//!         is_encrypted detection, encrypt_if_needed / decrypt_if_needed wrappers,
//!         None input handling.
//!
//! Does NOT cover: database-level field encryption (storage is tested by the
//!                 service integration tests which call the service layer).
//!
//! Protects: apps/backend/src/support/encrypt.rs

use bsds_backend::support::encrypt::{
    decrypt, decrypt_if_needed, encrypt, encrypt_if_needed, is_encrypted, EncryptError,
};

fn test_key() -> [u8; 32] {
    [0xABu8; 32]
}

fn alt_key() -> [u8; 32] {
    [0xCDu8; 32]
}

// ---------------------------------------------------------------------------
// Round-trip
// ---------------------------------------------------------------------------

#[test]
fn encrypt_then_decrypt_returns_original_plaintext() {
    let key = test_key();
    let plaintext = "hello world — this is a test";
    let encrypted = encrypt(plaintext, &key).unwrap();
    let decrypted = decrypt(&encrypted, &key).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn round_trip_preserves_unicode_and_special_characters() {
    let key = test_key();
    let plaintext = "नमस्ते 🌏 <>&\"'\\";
    let encrypted = encrypt(plaintext, &key).unwrap();
    let decrypted = decrypt(&encrypted, &key).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn round_trip_works_for_empty_string() {
    let key = test_key();
    let encrypted = encrypt("", &key).unwrap();
    let decrypted = decrypt(&encrypted, &key).unwrap();
    assert_eq!(decrypted, "");
}

// ---------------------------------------------------------------------------
// Output format
// ---------------------------------------------------------------------------

#[test]
fn encrypted_value_starts_with_enc_prefix() {
    let key = test_key();
    let result = encrypt("test", &key).unwrap();
    assert!(result.starts_with("enc:"), "should start with 'enc:' prefix");
}

#[test]
fn is_encrypted_returns_true_for_enc_prefixed_value() {
    assert!(is_encrypted("enc:some_base64_here"));
}

#[test]
fn is_encrypted_returns_false_for_plaintext() {
    assert!(!is_encrypted("plaintext"));
    assert!(!is_encrypted(""));
}

// ---------------------------------------------------------------------------
// IV randomisation
// ---------------------------------------------------------------------------

#[test]
fn same_plaintext_encrypts_to_different_ciphertexts() {
    let key = test_key();
    let a = encrypt("same input", &key).unwrap();
    let b = encrypt("same input", &key).unwrap();
    assert_ne!(a, b, "IV randomisation must produce different ciphertexts");
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn decrypt_plaintext_without_enc_prefix_returns_not_encrypted_error() {
    let key = test_key();
    let err = decrypt("plain value", &key).unwrap_err();
    assert!(
        matches!(err, EncryptError::NotEncrypted),
        "expected NotEncrypted error, got: {err}"
    );
}

#[test]
fn decrypt_with_wrong_key_returns_decryption_failed_error() {
    let key = test_key();
    let alt = alt_key();
    let encrypted = encrypt("secret data", &key).unwrap();
    let err = decrypt(&encrypted, &alt).unwrap_err();
    assert!(
        matches!(err, EncryptError::DecryptionFailed),
        "expected DecryptionFailed with wrong key, got: {err}"
    );
}

#[test]
fn decrypt_truncated_blob_returns_blob_too_short_error() {
    // Manually craft an enc: value with a base64 payload that is too short
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    let tiny = BASE64.encode(&[0u8; 5]); // 5 bytes — less than IV(12) + tag(16) = 28
    let fake_enc = format!("enc:{tiny}");
    let key = test_key();
    let err = decrypt(&fake_enc, &key).unwrap_err();
    assert!(
        matches!(err, EncryptError::BlobTooShort { .. }),
        "expected BlobTooShort error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// encrypt_if_needed
// ---------------------------------------------------------------------------

#[test]
fn encrypt_if_needed_encrypts_plaintext_value() {
    let key = test_key();
    let result = encrypt_if_needed(Some("plain"), &key).unwrap();
    let encrypted = result.unwrap();
    assert!(is_encrypted(&encrypted));
}

#[test]
fn encrypt_if_needed_skips_already_encrypted_value() {
    let key = test_key();
    let encrypted = encrypt("test", &key).unwrap();
    let result = encrypt_if_needed(Some(&encrypted), &key).unwrap().unwrap();
    assert_eq!(result, encrypted, "already-encrypted value should not be re-encrypted");
}

#[test]
fn encrypt_if_needed_returns_none_for_none_input() {
    let key = test_key();
    let result = encrypt_if_needed(None, &key).unwrap();
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// decrypt_if_needed
// ---------------------------------------------------------------------------

#[test]
fn decrypt_if_needed_decrypts_enc_prefixed_value() {
    let key = test_key();
    let encrypted = encrypt("decrypted text", &key).unwrap();
    let result = decrypt_if_needed(Some(&encrypted), &key).unwrap();
    assert_eq!(result, "decrypted text");
}

#[test]
fn decrypt_if_needed_returns_plaintext_unchanged() {
    let key = test_key();
    let result = decrypt_if_needed(Some("already plain"), &key).unwrap();
    assert_eq!(result, "already plain");
}

#[test]
fn decrypt_if_needed_returns_none_for_none_input() {
    let key = test_key();
    let result = decrypt_if_needed(None, &key);
    assert!(result.is_none());
}
