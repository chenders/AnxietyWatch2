//! XSalsa20-Poly1305 secretbox encryption/decryption.
//!
//! Uses the `crypto_secretbox` crate, which implements `NaCl`'s
//! `crypto_secretbox` construction (XSalsa20 stream cipher + Poly1305 MAC).

use crypto_secretbox::aead::{Aead, KeyInit};
use crypto_secretbox::{Key, Nonce, XSalsa20Poly1305};

/// Encrypt `plaintext` with the given key and nonce.
///
/// Returns the ciphertext with the 16-byte Poly1305 authentication tag
/// prepended (per the `NaCl` `crypto_secretbox` wire format).
///
/// # Errors
/// Returns an error when the plaintext is empty: `NaCl` forbids empty
/// messages, so the caller must represent absence differently (e.g. a JSON
/// `null` payload inside the envelope).
pub fn encrypt(
    plaintext: &[u8],
    key_bytes: &[u8; 32],
    nonce_bytes: &[u8; 24],
) -> Result<Vec<u8>, String> {
    let key = Key::from_slice(key_bytes);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = XSalsa20Poly1305::new(key);
    cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("encryption failed: {e}"))
}

/// Decrypt `ciphertext` (tag prepended) with the given key and nonce.
///
/// # Errors
/// Returns an error when the ciphertext is shorter than the 16-byte tag or
/// the tag fails verification — wrong key, wrong nonce, or tampered data.
pub fn decrypt(
    ciphertext: &[u8],
    key_bytes: &[u8; 32],
    nonce_bytes: &[u8; 24],
) -> Result<Vec<u8>, String> {
    let key = Key::from_slice(key_bytes);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = XSalsa20Poly1305::new(key);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("decryption failed: {e}"))
}
