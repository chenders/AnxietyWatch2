//! Portable `AirSense` 11 session cryptography.
//!
//! This module contains only the protocol mathematics and the bounded AES
//! envelope.  It does not know about `BlueZ`, credentials files, logging, or
//! pairing.  A caller supplies the private SRP value and the random IV; this
//! keeps the protocol implementation deterministic in tests without inventing
//! a random-number source in a transport-independent module.

use aes::Aes256;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::NoPadding};
use crypto_bigint::{
    Odd, U2048,
    modular::{FixedMontyForm, FixedMontyParams},
};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

/// The fixed-width SRP group element size used by `AirSense` 11.
pub const SRP_PAD_BYTES: usize = 256;
/// The SHA-256 digest size used by the `AirSense` 11 protocol.
pub const DIGEST_BYTES: usize = 32;
/// The largest salt accepted by the reference implementation.
pub const MAX_SALT_BYTES: usize = 64;
/// The largest challenge or nonce accepted by the reference implementation.
pub const MAX_NONCE_BYTES: usize = 32;
/// The largest encrypted JSON FIG payload sent in an application request.
pub const MAX_ENCRYPTED_REQUEST_BYTES: usize = 7650;
/// The largest encrypted JSON FIG payload accepted from an application response.
pub const MAX_ENCRYPTED_RESPONSE_BYTES: usize = 7682;
/// The largest JSON body that fits in [`MAX_ENCRYPTED_REQUEST_BYTES`].
pub const MAX_JSON_REQUEST_BYTES: usize = 7630;
/// The largest JSON body that fits in [`MAX_ENCRYPTED_RESPONSE_BYTES`].
pub const MAX_JSON_RESPONSE_BYTES: usize = 7662;

const SRP_MODULUS: Odd<U2048> = Odd::<U2048>::from_be_hex(concat!(
    "AC6BDB41324A9A9BF166DE5E1389582FAF72B6651987EE07FC3192943DB56050",
    "A37329CBB4A099ED8193E0757767A13DD52312AB4B03310DCD7F48A9DA04FD50",
    "E8083969EDB767B0CF6095179A163AB3661A05FBD5FAAAE82918A9962F0B93B",
    "855F97993EC975EEAA80D740ADBF4FF747359D041D5C33EA71D281E446B1477",
    "3BCA97B43A23FB801676BD207A436C6481F1D2B9078717461A5B9D32E688F87",
    "748544523B524B0D57D5EA77A2775D2ECFA032CFBDBF52FB3786160279004E5",
    "7AE6AF874E7303CE53299CCC041C7BC308D82A5698F3A8D0C38271AE35F8E9D",
    "BFBB694B5C803D89F7AE435DE236D525F54759B65E372FCD68EF20FA7111F9E",
    "4AFF73"
));
const SRP_PARAMS: FixedMontyParams<{ U2048::LIMBS }> = FixedMontyParams::new(SRP_MODULUS);
/// Errors produced before an `AirSense` cryptographic value may be used.
///
/// Error variants intentionally contain no key, challenge, nonce, proof, or
/// other protocol material.  They are safe to carry through a status path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    /// A private SRP value was zero.
    InvalidPrivateKey,
    /// The pairing passkey was not exactly four decimal ASCII digits.
    InvalidPasskey,
    /// A hexadecimal field was malformed or outside its fixed bound.
    InvalidHex,
    /// The server public key was not a valid non-zero group element.
    InvalidServerPublicKey,
    /// The salt was empty, malformed, or too large.
    InvalidSalt,
    /// The SRP arithmetic produced an invalid zero base or shared secret.
    InvalidArithmetic,
    /// The configured master pair key was absent or all zero.
    InvalidMasterKey,
    /// A session operation was attempted before a master key was configured.
    MasterKeyMissing,
    /// A challenge was empty, malformed, or too large.
    InvalidChallenge,
    /// A nonce was empty, malformed, or too large.
    InvalidNonce,
}

impl core::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::InvalidPrivateKey => "private_key_invalid",
            Self::InvalidPasskey => "passkey_invalid",
            Self::InvalidHex => "hex_invalid",
            Self::InvalidServerPublicKey => "server_public_key_invalid",
            Self::InvalidSalt => "salt_invalid",
            Self::InvalidArithmetic => "srp_arithmetic_invalid",
            Self::InvalidMasterKey => "master_key_invalid",
            Self::MasterKeyMissing => "master_key_missing",
            Self::InvalidChallenge => "challenge_invalid",
            Self::InvalidNonce => "nonce_invalid",
        };
        f.write_str(text)
    }
}

impl std::error::Error for CryptoError {}

/// Errors produced while handling the AES-CBC JSON envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeError {
    /// No authenticated session key has been derived.
    SessionKeyMissing,
    /// The operating-system random source could not produce an IV.
    RandomIvUnavailable,
    /// The JSON body was empty or not UTF-8.
    JsonInvalid,
    /// The JSON body exceeded the bounded envelope capacity.
    JsonTooLarge {
        /// The supplied JSON length.
        len: usize,
        /// The largest accepted JSON length.
        max: usize,
    },
    /// The encrypted envelope exceeded the bounded FIG payload capacity.
    EncryptedTooLarge {
        /// The supplied envelope length.
        len: usize,
        /// The largest accepted envelope length.
        max: usize,
    },
    /// The IV/ciphertext length was not an AES-CBC block shape.
    EncryptedLengthInvalid {
        /// The supplied envelope length.
        len: usize,
    },
    /// AES key/IV setup failed.
    CipherInitFailed,
    /// AES rejected the in-place operation.
    CipherOperationFailed,
    /// The decrypted two-byte length exceeded the decrypted body.
    PayloadLengthInvalid {
        /// The length declared inside the envelope.
        declared: usize,
        /// Bytes available after the length field.
        available: usize,
    },
    /// Bytes after the JSON body were not all zero.
    PaddingInvalid,
    /// The decrypted JSON body was empty.
    PayloadEmpty,
    /// The decrypted body was not UTF-8.
    Utf8Invalid,
}

impl core::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SessionKeyMissing => f.write_str("session_key_missing"),
            Self::RandomIvUnavailable => f.write_str("random_iv_unavailable"),
            Self::JsonInvalid => f.write_str("json_invalid"),
            Self::JsonTooLarge { len, max } => {
                write!(f, "json length {len} exceeds {max}-byte limit")
            }
            Self::EncryptedTooLarge { len, max } => {
                write!(f, "encrypted length {len} exceeds {max}-byte limit")
            }
            Self::EncryptedLengthInvalid { len } => {
                write!(f, "encrypted length {len} is not an AES-CBC envelope")
            }
            Self::CipherInitFailed => f.write_str("cipher_init_failed"),
            Self::CipherOperationFailed => f.write_str("cipher_operation_failed"),
            Self::PayloadLengthInvalid {
                declared,
                available,
            } => write!(
                f,
                "decrypted length {declared} exceeds {available}-byte body"
            ),
            Self::PaddingInvalid => f.write_str("decrypted_padding_invalid"),
            Self::PayloadEmpty => f.write_str("decrypted_payload_empty"),
            Self::Utf8Invalid => f.write_str("decrypted_utf8_invalid"),
        }
    }
}

impl std::error::Error for EnvelopeError {}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode_hex_exact<const N: usize>(text: &str) -> Result<[u8; N], CryptoError> {
    let bytes = text.as_bytes();
    if bytes.len() != N.saturating_mul(2) {
        return Err(CryptoError::InvalidHex);
    }

    let mut output = [0_u8; N];
    for (destination, pair) in output.iter_mut().zip(bytes.chunks_exact(2)) {
        let Some(high) = pair.first().and_then(|byte| hex_nibble(*byte)) else {
            return Err(CryptoError::InvalidHex);
        };
        let Some(low) = pair.get(1).and_then(|byte| hex_nibble(*byte)) else {
            return Err(CryptoError::InvalidHex);
        };
        *destination = (high << 4) | low;
    }
    if !bytes.chunks_exact(2).remainder().is_empty() {
        return Err(CryptoError::InvalidHex);
    }
    Ok(output)
}

fn decode_hex_bounded<const N: usize>(text: &str) -> Result<([u8; N], usize), CryptoError> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || !bytes.chunks_exact(2).remainder().is_empty() {
        return Err(CryptoError::InvalidHex);
    }
    let length = bytes.len() / 2;
    if length > N {
        return Err(CryptoError::InvalidHex);
    }

    let mut output = [0_u8; N];
    for (destination, pair) in output.iter_mut().zip(bytes.chunks_exact(2)) {
        let Some(high) = pair.first().and_then(|byte| hex_nibble(*byte)) else {
            return Err(CryptoError::InvalidHex);
        };
        let Some(low) = pair.get(1).and_then(|byte| hex_nibble(*byte)) else {
            return Err(CryptoError::InvalidHex);
        };
        *destination = (high << 4) | low;
    }
    Ok((output, length))
}

/// Encodes a bounded protocol value as uppercase hexadecimal.
#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    fn digit(value: u8) -> char {
        match value {
            0..=9 => char::from(b'0' + value),
            10..=15 => char::from(b'A' + value - 10),
            _ => '?',
        }
    }

    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(digit(*byte >> 4));
        output.push(digit(*byte & 0x0f));
    }
    output
}

fn padded_32(bytes: &[u8; DIGEST_BYTES]) -> [u8; SRP_PAD_BYTES] {
    let mut output = [0_u8; SRP_PAD_BYTES];
    for (destination, source) in output
        .iter_mut()
        .skip(SRP_PAD_BYTES - DIGEST_BYTES)
        .zip(bytes)
    {
        *destination = *source;
    }
    output
}

fn hash_parts(parts: &[&[u8]]) -> [u8; DIGEST_BYTES] {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part);
    }
    digest.finalize().into()
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Result<[u8; DIGEST_BYTES], CryptoError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|_| CryptoError::InvalidMasterKey)?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().into())
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0_u8;
    for (a, b) in left.iter().zip(right) {
        difference |= *a ^ *b;
    }
    difference == 0
}

fn big_from_hash(hash: &[u8; DIGEST_BYTES]) -> U2048 {
    U2048::from_be_slice(&padded_32(hash))
}

fn residue(value: &U2048) -> FixedMontyForm<{ U2048::LIMBS }> {
    FixedMontyForm::new(value, &SRP_PARAMS)
}

/// A deterministic SRP client state for the `AirSense` first-pairing formula.
///
/// Production reconnect does not invoke first pairing.  The type exists so the
/// fixed-width formula can be tested against synthetic values and so a future,
/// separately approved pairing tool can reuse a reviewed implementation.
pub struct SrpClient {
    private_key: [u8; DIGEST_BYTES],
    public_key: [u8; SRP_PAD_BYTES],
}

impl SrpClient {
    /// Starts SRP with a caller-provided 32-byte private value.
    ///
    /// A production pairing caller must obtain this value from a cryptographic
    /// random source.  Supplying it explicitly keeps this module portable and
    /// makes the test vector reproducible.
    ///
    /// # Errors
    /// Returns [`CryptoError::InvalidPrivateKey`] for an all-zero private
    /// value, or [`CryptoError::InvalidArithmetic`] if the public value is
    /// unexpectedly zero.
    pub fn begin(private_key: [u8; DIGEST_BYTES]) -> Result<Self, CryptoError> {
        if private_key.iter().all(|byte| *byte == 0) {
            return Err(CryptoError::InvalidPrivateKey);
        }

        let private_integer = U2048::from_be_slice(&padded_32(&private_key));
        let generator = residue(&U2048::from_u8(2));
        let public_key: [u8; SRP_PAD_BYTES] = generator
            .pow(&private_integer)
            .retrieve()
            .to_be_bytes()
            .into();
        if public_key.iter().all(|byte| *byte == 0) {
            return Err(CryptoError::InvalidArithmetic);
        }
        Ok(Self {
            private_key,
            public_key,
        })
    }

    /// Returns the fixed-width public key bytes sent in `StartKeyExchange`.
    #[must_use]
    pub const fn public_key(&self) -> &[u8; SRP_PAD_BYTES] {
        &self.public_key
    }

    /// Returns the fixed-width public key as uppercase hexadecimal.
    #[must_use]
    pub fn public_key_hex(&self) -> String {
        encode_hex(&self.public_key)
    }

    /// Completes the fixed-width `AirSense` SRP calculation.
    ///
    /// The returned master key is the reusable pairing credential.  This
    /// method never formats or logs it; the caller is responsible for keeping
    /// the returned proof private and for verifying the server proof before
    /// persisting any credential.
    ///
    /// # Errors
    /// Returns a [`CryptoError`] when the passkey, fixed-width server public
    /// key, salt, or SRP arithmetic is invalid.
    pub fn finish(
        &self,
        passkey: &[u8],
        server_public_key_hex: &str,
        salt_hex: &str,
    ) -> Result<SrpProof, CryptoError> {
        if passkey.len() != 4 || passkey.iter().any(|byte| *byte < b'0' || *byte > b'9') {
            return Err(CryptoError::InvalidPasskey);
        }
        let server_public_key = decode_hex_exact::<SRP_PAD_BYTES>(server_public_key_hex)
            .map_err(|_| CryptoError::InvalidServerPublicKey)?;
        let (salt, salt_length) =
            decode_hex_bounded::<MAX_SALT_BYTES>(salt_hex).map_err(|_| CryptoError::InvalidSalt)?;
        if salt_length == 0 {
            return Err(CryptoError::InvalidSalt);
        }
        let salt = salt.get(..salt_length).ok_or(CryptoError::InvalidSalt)?;

        let server_integer = U2048::from_be_slice(&server_public_key);
        let server_residue = residue(&server_integer);
        if server_residue.retrieve() == U2048::ZERO {
            return Err(CryptoError::InvalidServerPublicKey);
        }

        let modulus_bytes: [u8; SRP_PAD_BYTES] = SRP_MODULUS.get().to_be_bytes().into();
        let generator_integer = U2048::from_u8(2);
        let generator_bytes: [u8; SRP_PAD_BYTES] = generator_integer.to_be_bytes().into();
        let multiplier_hash = hash_parts(&[&modulus_bytes, &generator_bytes]);
        let passkey_hash = hash_parts(&[passkey]);
        let private_hash = hash_parts(&[salt, &passkey_hash]);
        let scrambling_hash = hash_parts(&[&self.public_key, &server_public_key]);

        let multiplier = big_from_hash(&multiplier_hash);
        let private_value = big_from_hash(&private_hash);
        let scrambling = big_from_hash(&scrambling_hash);
        if scrambling == U2048::ZERO {
            return Err(CryptoError::InvalidArithmetic);
        }

        let generator_residue = residue(&generator_integer);
        let generator_private = generator_residue.pow(&private_value);
        let multiplier_residue = residue(&multiplier);
        let multiplied_generator = multiplier_residue.mul(&generator_private);
        let base = server_residue.sub(&multiplied_generator);
        if base.retrieve() == U2048::ZERO {
            return Err(CryptoError::InvalidArithmetic);
        }

        let private_key_integer = U2048::from_be_slice(&padded_32(&self.private_key));
        let exponent = private_key_integer.wrapping_add(&scrambling.wrapping_mul(&private_value));
        let shared_secret: [u8; SRP_PAD_BYTES] =
            base.pow(&exponent).retrieve().to_be_bytes().into();
        if shared_secret.iter().all(|byte| *byte == 0) {
            return Err(CryptoError::InvalidArithmetic);
        }
        let master_key = hash_parts(&[&shared_secret]);

        let modulus_hash = hash_parts(&[&modulus_bytes]);
        let generator_hash = hash_parts(&[&generator_bytes]);
        let mut modulus_xor_generator = [0_u8; DIGEST_BYTES];
        for ((destination, modulus_byte), generator_byte) in modulus_xor_generator
            .iter_mut()
            .zip(modulus_hash)
            .zip(generator_hash)
        {
            *destination = modulus_byte ^ generator_byte;
        }

        let client_proof = hash_parts(&[
            &modulus_xor_generator,
            salt,
            &self.public_key,
            &server_public_key,
            &master_key,
        ]);
        let expected_server_proof = hash_parts(&[&self.public_key, &client_proof, &master_key]);
        Ok(SrpProof {
            client_proof,
            master_key,
            expected_server_proof,
        })
    }
}

impl Drop for SrpClient {
    fn drop(&mut self) {
        self.private_key.zeroize();
        self.public_key.zeroize();
    }
}

/// The proof and master key produced by one SRP exchange.
///
/// This type deliberately does not implement `Debug`, `Display`, or
/// serialization: formatting it would make it too easy to put pairing
/// material in a log or fixture.
pub struct SrpProof {
    client_proof: [u8; DIGEST_BYTES],
    master_key: [u8; DIGEST_BYTES],
    expected_server_proof: [u8; DIGEST_BYTES],
}

impl SrpProof {
    /// Returns the client proof bytes for `ConfirmKeyExchange`.
    #[must_use]
    pub const fn client_proof(&self) -> &[u8; DIGEST_BYTES] {
        &self.client_proof
    }

    /// Returns the derived reusable master pair key.
    #[must_use]
    pub const fn master_key(&self) -> &[u8; DIGEST_BYTES] {
        &self.master_key
    }

    /// Checks the device's fixed-width `M2` proof without early exit on bytes.
    #[must_use]
    pub fn verify_server(&self, server_proof: &[u8; DIGEST_BYTES]) -> bool {
        constant_time_equal(&self.expected_server_proof, server_proof)
    }

    /// Checks an uppercase or lowercase hexadecimal device `M2` proof.
    #[must_use]
    pub fn verify_server_hex(&self, server_proof_hex: &str) -> bool {
        let Ok(server_proof) = decode_hex_exact::<DIGEST_BYTES>(server_proof_hex) else {
            return false;
        };
        self.verify_server(&server_proof)
    }
}

impl Drop for SrpProof {
    fn drop(&mut self) {
        self.client_proof.zeroize();
        self.master_key.zeroize();
        self.expected_server_proof.zeroize();
    }
}

/// Authenticated `AirSense` session cryptography.
///
/// A master pair key may be configured once, but an application session key is
/// cleared before every nonce derivation.  Callers should derive it only after
/// the device has accepted `CheckSessionIntegrity`.
pub struct SessionCrypto {
    master_key: [u8; DIGEST_BYTES],
    session_key: [u8; DIGEST_BYTES],
    master_key_ready: bool,
    session_key_ready: bool,
}

impl SessionCrypto {
    /// Creates session state from a validated 32-byte master pair key.
    ///
    /// # Errors
    /// Returns [`CryptoError::InvalidMasterKey`] when the key is all zero.
    pub fn new(master_key: [u8; DIGEST_BYTES]) -> Result<Self, CryptoError> {
        if master_key.iter().all(|byte| *byte == 0) {
            return Err(CryptoError::InvalidMasterKey);
        }
        Ok(Self {
            master_key,
            session_key: [0_u8; DIGEST_BYTES],
            master_key_ready: true,
            session_key_ready: false,
        })
    }

    /// Creates session state from the required 64-character hexadecimal key.
    ///
    /// # Errors
    /// Returns [`CryptoError::InvalidMasterKey`] for malformed, wrong-sized,
    /// or all-zero key material.
    pub fn from_master_key_hex(text: &str) -> Result<Self, CryptoError> {
        let key =
            decode_hex_exact::<DIGEST_BYTES>(text).map_err(|_| CryptoError::InvalidMasterKey)?;
        Self::new(key)
    }

    /// Replaces the master pair key and clears any derived session key.
    ///
    /// # Errors
    /// Returns [`CryptoError::InvalidMasterKey`] for malformed, wrong-sized,
    /// or all-zero key material.
    pub fn set_master_key_hex(&mut self, text: &str) -> Result<(), CryptoError> {
        self.clear();
        let key =
            decode_hex_exact::<DIGEST_BYTES>(text).map_err(|_| CryptoError::InvalidMasterKey)?;
        if key.iter().all(|byte| *byte == 0) {
            return Err(CryptoError::InvalidMasterKey);
        }
        self.master_key = key;
        self.master_key_ready = true;
        Ok(())
    }

    /// Reports whether a master pair key is present.
    #[must_use]
    pub const fn has_master_key(&self) -> bool {
        self.master_key_ready
    }

    /// Reports whether a fresh session key has been derived.
    #[must_use]
    pub const fn ready(&self) -> bool {
        self.session_key_ready
    }

    /// Computes `HMAC-SHA256(master_pair_key, challenge)` from a bounded hex challenge.
    ///
    /// # Errors
    /// Returns a [`CryptoError`] when the master key is absent or the challenge
    /// is malformed, empty, or outside its fixed bound.
    pub fn integrity_response(
        &self,
        challenge_hex: &str,
    ) -> Result<[u8; DIGEST_BYTES], CryptoError> {
        if !self.master_key_ready {
            return Err(CryptoError::MasterKeyMissing);
        }
        let (challenge, length) = decode_hex_bounded::<MAX_NONCE_BYTES>(challenge_hex)
            .map_err(|_| CryptoError::InvalidChallenge)?;
        let challenge = challenge
            .get(..length)
            .ok_or(CryptoError::InvalidChallenge)?;
        if challenge.is_empty() {
            return Err(CryptoError::InvalidChallenge);
        }
        hmac_sha256(&self.master_key, challenge).map_err(|_| CryptoError::InvalidChallenge)
    }

    /// Computes the challenge response and formats it as uppercase hexadecimal.
    ///
    /// # Errors
    /// Returns the same errors as [`Self::integrity_response`].
    pub fn integrity_response_hex(&self, challenge_hex: &str) -> Result<String, CryptoError> {
        let response = self.integrity_response(challenge_hex)?;
        Ok(encode_hex(&response))
    }

    /// Derives `SHA256(master_pair_key || nonce_raw)` from a bounded hex nonce.
    ///
    /// # Errors
    /// Returns a [`CryptoError`] when the master key is absent or the nonce is
    /// malformed, empty, or outside its fixed bound.
    pub fn derive_session_key(&mut self, nonce_hex: &str) -> Result<(), CryptoError> {
        self.clear_session();
        if !self.master_key_ready {
            return Err(CryptoError::MasterKeyMissing);
        }
        let (nonce, length) = decode_hex_bounded::<MAX_NONCE_BYTES>(nonce_hex)
            .map_err(|_| CryptoError::InvalidNonce)?;
        let nonce = nonce.get(..length).ok_or(CryptoError::InvalidNonce)?;
        if nonce.is_empty() {
            return Err(CryptoError::InvalidNonce);
        }
        self.session_key = hash_parts(&[&self.master_key, nonce]);
        self.session_key_ready = true;
        Ok(())
    }

    /// Clears only the derived session key.
    pub fn clear_session(&mut self) {
        self.session_key.zeroize();
        self.session_key_ready = false;
    }

    /// Clears both the master key and the derived session key.
    pub fn clear(&mut self) {
        self.clear_session();
        self.master_key.zeroize();
        self.master_key_ready = false;
    }

    /// Encrypts a UTF-8 JSON body with an operating-system random IV.
    ///
    /// The resulting bytes are `[iv][AES-CBC(length || JSON || zero padding)]`.
    ///
    /// # Errors
    /// Returns an [`EnvelopeError`] when the session is absent, JSON is empty,
    /// invalid UTF-8, outside the fixed envelope bound, or the OS random source
    /// cannot provide an IV.
    pub fn encrypt_json(&self, json: &str) -> Result<Vec<u8>, EnvelopeError> {
        self.encrypt(json.as_bytes())
    }

    /// Encrypts a bounded UTF-8 JSON body with an operating-system random IV.
    ///
    /// # Errors
    /// Returns an [`EnvelopeError`] when the session is absent, JSON is empty,
    /// invalid UTF-8, outside the fixed envelope bound, or the OS random source
    /// cannot provide an IV.
    pub fn encrypt(&self, json: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        if !self.session_key_ready {
            return Err(EnvelopeError::SessionKeyMissing);
        }
        let mut iv = [0_u8; 16];
        getrandom::fill(&mut iv).map_err(|_| EnvelopeError::RandomIvUnavailable)?;
        self.encrypt_with_iv(json, iv)
    }

    /// Encrypts a UTF-8 JSON body with a caller-provided IV.
    ///
    /// This deterministic seam is intended for protocol vectors and tests.  A
    /// production caller should use [`Self::encrypt_json`] so the IV comes
    /// from the OS random source.
    ///
    /// # Errors
    /// Returns an [`EnvelopeError`] when the session is absent, JSON is empty,
    /// invalid UTF-8, or outside the fixed envelope bound.
    pub fn encrypt_json_with_iv(&self, json: &str, iv: [u8; 16]) -> Result<Vec<u8>, EnvelopeError> {
        self.encrypt_with_iv(json.as_bytes(), iv)
    }

    /// Encrypts a bounded UTF-8 JSON body with a caller-provided IV.
    ///
    /// This deterministic seam is intended for protocol vectors and tests.  A
    /// production caller should use [`Self::encrypt`] so the IV comes from the
    /// OS random source.
    ///
    /// # Errors
    /// Returns an [`EnvelopeError`] when the session is absent, JSON is empty,
    /// invalid UTF-8, or outside the fixed envelope bound.
    pub fn encrypt_with_iv(&self, json: &[u8], iv: [u8; 16]) -> Result<Vec<u8>, EnvelopeError> {
        if !self.session_key_ready {
            return Err(EnvelopeError::SessionKeyMissing);
        }
        if json.is_empty() || core::str::from_utf8(json).is_err() {
            return Err(EnvelopeError::JsonInvalid);
        }
        if json.len() > MAX_JSON_REQUEST_BYTES {
            return Err(EnvelopeError::JsonTooLarge {
                len: json.len(),
                max: MAX_JSON_REQUEST_BYTES,
            });
        }
        let json_length = u16::try_from(json.len()).map_err(|_| EnvelopeError::JsonTooLarge {
            len: json.len(),
            max: MAX_JSON_REQUEST_BYTES,
        })?;
        let framed_length = 2_usize + json.len();
        let padded_length = framed_length.div_ceil(16) * 16;
        let encrypted_length = 16_usize + padded_length;
        if encrypted_length > MAX_ENCRYPTED_REQUEST_BYTES {
            return Err(EnvelopeError::EncryptedTooLarge {
                len: encrypted_length,
                max: MAX_ENCRYPTED_REQUEST_BYTES,
            });
        }

        let mut output = vec![0_u8; encrypted_length];
        let (output_iv, ciphertext) = output.split_at_mut(16);
        output_iv.copy_from_slice(&iv);
        let (length_field, json_and_padding) = ciphertext.split_at_mut(2);
        length_field.copy_from_slice(&json_length.to_le_bytes());
        let Some(destination) = json_and_padding.get_mut(..json.len()) else {
            return Err(EnvelopeError::CipherOperationFailed);
        };
        destination.copy_from_slice(json);

        let cipher = cbc::Encryptor::<Aes256>::new_from_slices(&self.session_key, &iv)
            .map_err(|_| EnvelopeError::CipherInitFailed)?;
        cipher
            .encrypt_padded_mut::<NoPadding>(ciphertext, padded_length)
            .map_err(|_| EnvelopeError::CipherOperationFailed)?;
        Ok(output)
    }

    /// Decrypts and validates a bounded `AirSense` AES-CBC JSON envelope.
    ///
    /// # Errors
    /// Returns an [`EnvelopeError`] for an absent session key, invalid block
    /// shape or ciphertext, bad embedded length, non-zero padding, empty JSON,
    /// or invalid UTF-8.
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        if !self.session_key_ready {
            return Err(EnvelopeError::SessionKeyMissing);
        }
        if encrypted.len() > MAX_ENCRYPTED_RESPONSE_BYTES {
            return Err(EnvelopeError::EncryptedTooLarge {
                len: encrypted.len(),
                max: MAX_ENCRYPTED_RESPONSE_BYTES,
            });
        }
        if encrypted.len() < 32 || !(encrypted.len() - 16).is_multiple_of(16) {
            return Err(EnvelopeError::EncryptedLengthInvalid {
                len: encrypted.len(),
            });
        }
        let iv = encrypted
            .get(..16)
            .ok_or(EnvelopeError::EncryptedLengthInvalid {
                len: encrypted.len(),
            })?;
        let ciphertext = encrypted
            .get(16..)
            .ok_or(EnvelopeError::EncryptedLengthInvalid {
                len: encrypted.len(),
            })?;
        let mut plaintext = Zeroizing::new(ciphertext.to_vec());
        let cipher = cbc::Decryptor::<Aes256>::new_from_slices(&self.session_key, iv)
            .map_err(|_| EnvelopeError::CipherInitFailed)?;
        cipher
            .decrypt_padded_mut::<NoPadding>(plaintext.as_mut_slice())
            .map_err(|_| EnvelopeError::CipherOperationFailed)?;

        let header = plaintext
            .get(..2)
            .ok_or(EnvelopeError::PayloadLengthInvalid {
                declared: 0,
                available: 0,
            })?;
        let declared = match (header.first(), header.get(1)) {
            (Some(low), Some(high)) => usize::from(u16::from_le_bytes([*low, *high])),
            _ => {
                return Err(EnvelopeError::PayloadLengthInvalid {
                    declared: 0,
                    available: 0,
                });
            }
        };
        let available = plaintext.len().saturating_sub(2);
        if declared > available {
            return Err(EnvelopeError::PayloadLengthInvalid {
                declared,
                available,
            });
        }
        let end = 2_usize + declared;
        let payload = plaintext
            .get(2..end)
            .ok_or(EnvelopeError::PayloadLengthInvalid {
                declared,
                available,
            })?;
        let padding = plaintext.get(end..).ok_or(EnvelopeError::PaddingInvalid)?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(EnvelopeError::PaddingInvalid);
        }
        if payload.is_empty() {
            return Err(EnvelopeError::PayloadEmpty);
        }
        if core::str::from_utf8(payload).is_err() {
            return Err(EnvelopeError::Utf8Invalid);
        }
        Ok(payload.to_vec())
    }
}

impl Drop for SessionCrypto {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixed public vectors and assertion diagnostics are test-only"
)]
mod tests {
    use super::*;

    const SERVER_PUBLIC_KEY_HEX: &str = "1FA650917C4F8AB283FE1D8D5BD38F1EF9196C0543A54EA44302F5BAEB16F8B6F2B9EE1EA31E56319DA48936BB1F2286A939905729BED6930A208F020456227F0EEB751AD972C9E98737B07C7B79B3AB3340F9F6E29111635F54A90F0ED07B08B5333EC52BA83F5CB7B78F1560E911DC8BEBC0A791408695BCC8F46F18E65C34726C8BD0F35110D25D1E2223CEAF2471C3808C4FC31C67EF1CCA9BE9B49E64A887C6E61364AC79F723D87AE6F47F8A727FCC9416375651FFC3B18FB5A6BB4227D352680EE489B2EB9724B0586A87B89306ACBE80FD9BF7ADF6355BC13CD0A0BC7977522E10478C3FA822DAE138AA03CF34745CD347E8163EFD1D1617976D6E53";
    const SALT_HEX: &str = "101112131415161718191A1B1C1D1E1F";

    fn private_key() -> [u8; DIGEST_BYTES] {
        let mut key = [0_u8; DIGEST_BYTES];
        for (byte, value) in key.iter_mut().zip(0_u8..=31) {
            *byte = value;
        }
        key
    }

    #[test]
    fn srp_matches_fixed_width_synthetic_vector() {
        let client = SrpClient::begin(private_key()).expect("private key");
        assert_eq!(
            client.public_key_hex(),
            "47118709BE0DE1160A63B14B0524AE22270F78B0120855C88F285F2EB193E4727A79E25E136ECFC7CF3ECD0CDAAAFEFCF6A93C29DAE8A76A9B9E6C044B3AF05A47FE7122F579AC62BD7C8263B832139AE29DEB5258B91A7F64FCF073E1800C3819E83161B3DCDBBD116680CBFEFF2F25E06BE14F654E0E274B71EB3DCE95207367C319F02F0562A8FD26DB4837DA5BE55C6E367A6642BDB2A82DECAB0C4E768671FBD8D924774729172F848E72913D0E842FB5ABB25BDEA4D43AC7502CFB3DB6EB980C3F469475D707E5ADF40137E74BC9D1AC072C764108FFBA19DD917A018994FC967E840C4A6665A04933927589CB3E7761513D41B8A50E2314684364D465"
        );
        let proof = client
            .finish(b"1234", SERVER_PUBLIC_KEY_HEX, SALT_HEX)
            .expect("SRP vector");
        assert_eq!(
            encode_hex(proof.client_proof()),
            "03C0C32C1BE48A85CFABF8E056CBFC3E62731D0F382AF2399394179ADC8A587E"
        );
        assert_eq!(
            encode_hex(proof.master_key()),
            "8C1A7DCBB74EE8903186F8F063A97952FE2B29C087C70F30D209573CACE8F788"
        );
        assert!(
            proof.verify_server_hex(
                "038352C4FB8B22FF31C7945F366325C71B6F33BBFA4CCDC8B2699F5922AE9B47"
            )
        );
        assert!(
            !proof.verify_server_hex(
                "038352C4FB8B22FF31C7945F366325C71B6F33BBFA4CCDC8B2699F5922AE9B40"
            )
        );
    }

    #[test]
    fn srp_rejects_bad_inputs_without_falling_back() {
        assert_eq!(
            SrpClient::begin([0_u8; DIGEST_BYTES])
                .err()
                .expect("zero private key"),
            CryptoError::InvalidPrivateKey
        );
        let client = SrpClient::begin(private_key()).expect("private key");
        assert_eq!(
            client
                .finish(b"12", SERVER_PUBLIC_KEY_HEX, SALT_HEX)
                .err()
                .expect("passkey"),
            CryptoError::InvalidPasskey
        );
        assert_eq!(
            client
                .finish(b"1234", &"00".repeat(SRP_PAD_BYTES), SALT_HEX)
                .err()
                .expect("zero server key"),
            CryptoError::InvalidServerPublicKey
        );
        assert_eq!(
            client
                .finish(b"1234", SERVER_PUBLIC_KEY_HEX, "")
                .err()
                .expect("empty salt"),
            CryptoError::InvalidSalt
        );
    }

    #[test]
    fn challenge_and_envelope_match_public_vector() {
        let mut crypto = SessionCrypto::from_master_key_hex(
            "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F",
        )
        .expect("master key");
        assert_eq!(
            crypto
                .integrity_response_hex(
                    "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F"
                )
                .expect("HMAC"),
            "E8499BE4F1980D68F13222A418DF5CBD97D53FDDF590C2108E22D40005B70713"
        );
        crypto
            .derive_session_key("202122232425262728292A2B2C2D2E2F303132333435363738393A3B3C3D3E3F")
            .expect("nonce");
        assert!(crypto.ready());
        let encrypted = crypto
            .encrypt_json_with_iv(
                r#"{"ok":true}"#,
                [
                    0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB, 0xAC,
                    0xAD, 0xAE, 0xAF,
                ],
            )
            .expect("encrypt");
        assert_eq!(
            encode_hex(&encrypted),
            "A0A1A2A3A4A5A6A7A8A9AAABACADAEAF8FBDB1C3167CF862FBD9D2D0AD04A273"
        );
        assert_eq!(
            crypto.decrypt(&encrypted).expect("decrypt"),
            br#"{"ok":true}"#
        );
        assert_eq!(
            crypto
                .decrypt(&encrypted[..31])
                .expect_err("truncated envelope"),
            EnvelopeError::EncryptedLengthInvalid { len: 31 }
        );
        let mut corrupted = encrypted;
        if let Some(byte) = corrupted.last_mut() {
            *byte ^= 1;
        }
        assert!(crypto.decrypt(&corrupted).is_err());
        crypto.clear_session();
        assert_eq!(
            crypto
                .encrypt_json_with_iv("{}", [0_u8; 16])
                .expect_err("cleared session"),
            EnvelopeError::SessionKeyMissing
        );
    }

    #[test]
    fn an_invalid_master_key_replacement_clears_existing_session() {
        let mut crypto = SessionCrypto::new([1_u8; DIGEST_BYTES]).expect("master key");
        crypto.derive_session_key("01").expect("nonce");
        assert!(matches!(
            crypto.set_master_key_hex("not-a-key"),
            Err(CryptoError::InvalidMasterKey)
        ));
        assert!(!crypto.has_master_key());
        assert!(!crypto.ready());
    }

    #[test]
    fn envelope_bounds_are_explicit() {
        let mut crypto = SessionCrypto::new([1_u8; DIGEST_BYTES]).expect("master key");
        crypto.derive_session_key("01").expect("nonce");
        assert_eq!(
            crypto
                .encrypt_with_iv(&vec![b'a'; MAX_JSON_REQUEST_BYTES + 1], [0_u8; 16])
                .expect_err("oversize JSON"),
            EnvelopeError::JsonTooLarge {
                len: MAX_JSON_REQUEST_BYTES + 1,
                max: MAX_JSON_REQUEST_BYTES,
            }
        );
        assert_eq!(
            crypto.decrypt(&vec![0_u8; MAX_ENCRYPTED_RESPONSE_BYTES + 1]),
            Err(EnvelopeError::EncryptedTooLarge {
                len: MAX_ENCRYPTED_RESPONSE_BYTES + 1,
                max: MAX_ENCRYPTED_RESPONSE_BYTES,
            })
        );
    }

    #[test]
    fn empty_hex_is_not_accepted_for_protocol_fields() {
        let crypto = SessionCrypto::new([1_u8; DIGEST_BYTES]).expect("master key");
        assert_eq!(
            crypto.integrity_response("").expect_err("empty challenge"),
            CryptoError::InvalidChallenge
        );
    }
}
