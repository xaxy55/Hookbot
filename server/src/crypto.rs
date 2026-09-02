//! Encryption for third-party credentials held at rest.
//!
//! These are tokens the server must replay to an external service (a Hue
//! bridge, for example), so they cannot be hashed — hashing is one-way and the
//! server would no longer be able to use them. Instead they are encrypted with
//! a key derived from `session_secret`, and the routes never return them: the
//! API exposes only whether a credential is present.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use sha2::{Digest, Sha256};

/// Derive a key distinct from any other use of the session secret, so that
/// rotating one purpose cannot silently weaken another.
fn cipher(secret: &[u8; 32]) -> Aes256Gcm {
    let mut hasher = Sha256::new();
    hasher.update(b"hookbot:secret-at-rest:v1");
    hasher.update(secret);
    let derived = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&derived);
    Aes256Gcm::new(key)
}

/// Returns base64(nonce || ciphertext). A fresh random nonce per call is
/// required: reusing one under the same key breaks GCM badly.
pub fn encrypt(secret: &[u8; 32], plaintext: &str) -> Result<String, String> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher(secret)
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| "encryption failed".to_string())?;
    let mut blob = nonce.to_vec();
    blob.extend_from_slice(&ct);
    Ok(STANDARD.encode(blob))
}

pub fn decrypt(secret: &[u8; 32], blob: &str) -> Result<String, String> {
    let raw = STANDARD.decode(blob).map_err(|_| "not valid base64".to_string())?;
    if raw.len() < 12 {
        return Err("ciphertext too short".into());
    }
    let (nonce_bytes, ct) = raw.split_at(12);
    let plain = cipher(secret)
        .decrypt(Nonce::from_slice(nonce_bytes), ct)
        .map_err(|_| "decryption failed — wrong key or tampered data".to_string())?;
    String::from_utf8(plain).map_err(|_| "decrypted value is not UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: [u8; 32] = [3u8; 32];

    #[test]
    fn round_trips() {
        let blob = encrypt(&SECRET, "hue-token-abc").unwrap();
        assert_eq!(decrypt(&SECRET, &blob).unwrap(), "hue-token-abc");
    }

    #[test]
    fn ciphertext_does_not_contain_the_plaintext() {
        let blob = encrypt(&SECRET, "hue-token-abc").unwrap();
        assert!(!blob.contains("hue-token-abc"));
    }

    #[test]
    fn a_different_key_cannot_decrypt() {
        let blob = encrypt(&SECRET, "hue-token-abc").unwrap();
        assert!(decrypt(&[9u8; 32], &blob).is_err());
    }

    #[test]
    fn tampering_is_detected() {
        let blob = encrypt(&SECRET, "hue-token-abc").unwrap();
        let mut raw = STANDARD.decode(&blob).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0xff;
        assert!(decrypt(&SECRET, &STANDARD.encode(raw)).is_err());
    }

    #[test]
    fn same_plaintext_encrypts_differently_each_time() {
        // Confirms the nonce is fresh per call rather than fixed.
        let a = encrypt(&SECRET, "same").unwrap();
        let b = encrypt(&SECRET, "same").unwrap();
        assert_ne!(a, b);
    }
}
