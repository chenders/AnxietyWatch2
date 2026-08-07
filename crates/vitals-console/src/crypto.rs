//! XSalsa20-Poly1305 secretbox encryption/decryption.
//!
//! Uses the `crypto_secretbox` crate, which implements `NaCl`'s
//! `crypto_secretbox` construction (`XSalsa20` stream cipher + `Poly1305` MAC).

use crypto_secretbox::aead::{Aead, KeyInit};
use crypto_secretbox::{Key, Nonce, XSalsa20Poly1305};

/// Encrypt `plaintext` with the given key and nonce.
///
/// Returns the ciphertext with the 16-byte `Poly1305` authentication tag
/// prepended (per the `NaCl` `crypto_secretbox` wire format).
///
/// # Errors
/// Returns an error string if `plaintext` is empty — the `NaCl` secretbox
/// construction forbids encrypting empty messages.
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
/// Returns an error string if `ciphertext` is shorter than the 16-byte
/// authentication tag, or if the tag does not verify (wrong key, wrong nonce,
/// or tampered ciphertext).
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
