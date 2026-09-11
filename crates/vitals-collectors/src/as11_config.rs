//! External `AirSense` 11 configuration and Linux-address normalization.
//!
//! The collector never pairs and never manufactures a target address.  An
//! operator supplies a private JSON file containing the already-established
//! credential and either a six-octet Linux address or an opaque address with an
//! explicit Linux mapping.  This module is portable so the file and address
//! policy can be tested without `BlueZ`.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    as11_crypto::SessionCrypto,
    as11_rpc::{MAX_CLIENT_ID_BYTES, RpcRequest},
};

/// The default operator-managed credential location.
pub const DEFAULT_CREDENTIALS_FILE: &str = ".air11/air11-credentials.json";
/// Maximum credential-file size accepted before JSON parsing begins.
pub const MAX_CREDENTIAL_FILE_BYTES: u64 = 4096;

/// A normalized, externally provisioned `AirSense` credential.
///
/// This type intentionally has no `Debug` implementation.  Its fields include
/// the reusable master pair key and client identifier; callers should pass
/// accessors directly to the authentication layer and never serialize this
/// value into status.
pub struct Air11Credentials {
    target: String,
    client_id: String,
    master_pair_key: String,
}

impl Drop for Air11Credentials {
    fn drop(&mut self) {
        self.client_id.zeroize();
        self.master_pair_key.zeroize();
    }
}

impl Air11Credentials {
    /// Returns the normalized uppercase colon-separated Linux target.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Returns the configured client identifier.
    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Returns the configured 64-character hexadecimal master pair key.
    #[must_use]
    pub fn master_pair_key(&self) -> &str {
        &self.master_pair_key
    }
}

/// A credential-file validation failure.  Variants deliberately carry no
/// credential text, opaque UUID, or key material.
#[derive(Debug)]
pub enum CredentialsError {
    /// The file could not be inspected or read.
    Io(io::Error),
    /// The path exists but is not a regular file.
    NotRegularFile,
    /// The file is too large to parse under the fixed bound.
    FileTooLarge {
        /// The observed file length.
        len: u64,
        /// The maximum accepted file length.
        max: u64,
    },
    /// Group or other users can read or modify the credential file.
    InsecurePermissions,
    /// The bounded bytes were not the expected JSON object.
    JsonInvalid,
    /// The file did not contain a usable target address.
    Address(AddressError),
    /// The client identifier failed the request-builder bound.
    ClientIdInvalid,
    /// The master pair key failed fixed-width validation.
    MasterKeyInvalid,
    /// The process has no home directory for the default path.
    HomeDirectoryMissing,
}

impl core::fmt::Display for CredentialsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "credential_file_io: {error}"),
            Self::NotRegularFile => f.write_str("credential_file_not_regular"),
            Self::FileTooLarge { len, max } => {
                write!(f, "credential_file_length={len} exceeds limit={max}")
            }
            Self::InsecurePermissions => f.write_str("credential_file_permissions_not_private"),
            Self::JsonInvalid => f.write_str("credential_file_json_invalid"),
            Self::Address(error) => write!(f, "credential_address_{error}"),
            Self::ClientIdInvalid => f.write_str("credential_client_id_invalid"),
            Self::MasterKeyInvalid => f.write_str("credential_master_key_invalid"),
            Self::HomeDirectoryMissing => f.write_str("credential_home_directory_missing"),
        }
    }
}

impl std::error::Error for CredentialsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Address(error) => Some(error),
            _ => None,
        }
    }
}

/// Why an operator-supplied target was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressError {
    /// The target string was empty.
    Empty,
    /// The target was not one of the accepted six-octet forms.
    NotSixOctets,
    /// The target is an opaque `CoreBluetooth` UUID and has no Linux mapping.
    OpaqueCoreBluetoothUuidNeedsMapping,
    /// The six-octet target was all zeroes.
    AllZero,
}

impl core::fmt::Display for AddressError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reason = match self {
            Self::Empty => "empty",
            Self::NotSixOctets => "not_six_octets",
            Self::OpaqueCoreBluetoothUuidNeedsMapping => {
                "opaque_corebluetooth_uuid_needs_linux_mapping"
            }
            Self::AllZero => "all_zero",
        };
        f.write_str(reason)
    }
}

impl std::error::Error for AddressError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCredentials {
    #[serde(alias = "device_address", alias = "target")]
    address: String,
    #[serde(
        default,
        alias = "linuxAddress",
        alias = "linux_mac",
        alias = "linuxMac"
    )]
    linux_address: Option<String>,
    #[serde(alias = "clientId")]
    client_id: String,
    #[serde(
        alias = "masterPairKey",
        alias = "master_key",
        alias = "master_key_hex"
    )]
    master_pair_key: String,
}

impl Drop for RawCredentials {
    fn drop(&mut self) {
        self.address.zeroize();
        if let Some(address) = &mut self.linux_address {
            address.zeroize();
        }
        self.client_id.zeroize();
        self.master_pair_key.zeroize();
    }
}

/// Resolves `$HOME/.air11/air11-credentials.json` without putting the secret
/// in an environment variable or command-line argument.
///
/// # Errors
/// Returns [`CredentialsError::HomeDirectoryMissing`] when `HOME` is absent.
pub fn default_credentials_path() -> Result<PathBuf, CredentialsError> {
    let home = std::env::var_os("HOME").ok_or(CredentialsError::HomeDirectoryMissing)?;
    Ok(PathBuf::from(home).join(DEFAULT_CREDENTIALS_FILE))
}

/// Loads and validates an operator-managed credential file.
///
/// The metadata length and private-file check happen before the bytes are
/// parsed.  The master key is validated by the portable crypto implementation;
/// this function never logs or includes it in an error.
///
/// # Errors
/// Returns a named validation or I/O error when the file is missing, too large,
/// not private, malformed, or contains invalid credentials.
pub fn load_credentials(path: &Path) -> Result<Air11Credentials, CredentialsError> {
    let metadata = fs::symlink_metadata(path).map_err(CredentialsError::Io)?;
    if !metadata.is_file() {
        return Err(CredentialsError::NotRegularFile);
    }
    if metadata.len() > MAX_CREDENTIAL_FILE_BYTES {
        return Err(CredentialsError::FileTooLarge {
            len: metadata.len(),
            max: MAX_CREDENTIAL_FILE_BYTES,
        });
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(CredentialsError::InsecurePermissions);
        }
    }

    let bytes = Zeroizing::new(fs::read(path).map_err(CredentialsError::Io)?);
    let byte_len = u64::try_from(bytes.len()).map_err(|_| CredentialsError::FileTooLarge {
        len: u64::MAX,
        max: MAX_CREDENTIAL_FILE_BYTES,
    })?;
    if byte_len > MAX_CREDENTIAL_FILE_BYTES {
        return Err(CredentialsError::FileTooLarge {
            len: byte_len,
            max: MAX_CREDENTIAL_FILE_BYTES,
        });
    }
    let raw: RawCredentials =
        serde_json::from_slice(&bytes).map_err(|_| CredentialsError::JsonInvalid)?;

    let target = normalize_linux_target(&raw.address, raw.linux_address.as_deref())
        .map_err(CredentialsError::Address)?;
    if raw.client_id.is_empty() || raw.client_id.len() > MAX_CLIENT_ID_BYTES {
        return Err(CredentialsError::ClientIdInvalid);
    }
    // Exercise the same typed request bound used at the final RPC gate.  The
    // request is dropped immediately and its payload is zeroized on drop.
    RpcRequest::request_session(1, &raw.client_id)
        .map_err(|_| CredentialsError::ClientIdInvalid)?;
    SessionCrypto::from_master_key_hex(&raw.master_pair_key)
        .map_err(|_| CredentialsError::MasterKeyInvalid)?;

    Ok(Air11Credentials {
        target,
        client_id: raw.client_id.clone(),
        master_pair_key: raw.master_pair_key.clone(),
    })
}

/// Normalizes a direct address or an explicitly supplied Linux mapping.
///
/// Accepted direct forms are `AA:BB:CC:DD:EE:FF`, `AA-BB-CC-DD-EE-FF`, and
/// `AABBCCDDEEFF`.  A 128-bit `CoreBluetooth` UUID is never truncated, hashed,
/// or guessed into a MAC.  It is accepted only when `linux_mapping` supplies a
/// separate valid six-octet target.
///
/// # Errors
/// Returns [`AddressError`] when neither the direct address nor the explicit
/// mapping is a valid non-zero six-octet form.
pub fn normalize_linux_target(
    address: &str,
    linux_mapping: Option<&str>,
) -> Result<String, AddressError> {
    if let Some(mapping) = linux_mapping {
        return normalize_mac(mapping);
    }
    match normalize_mac(address) {
        Ok(normalized) => Ok(normalized),
        Err(_) if is_corebluetooth_uuid(address) => {
            Err(AddressError::OpaqueCoreBluetoothUuidNeedsMapping)
        }
        Err(error) => Err(error),
    }
}

fn normalize_mac(value: &str) -> Result<String, AddressError> {
    if value.is_empty() {
        return Err(AddressError::Empty);
    }
    let bytes = value.as_bytes();
    let mut octets = [0_u8; 6];
    match bytes.len() {
        12 => {
            for (index, octet) in octets.iter_mut().enumerate() {
                let begin = index.checked_mul(2).ok_or(AddressError::NotSixOctets)?;
                *octet = parse_hex_pair(bytes.get(begin..begin + 2))?;
            }
        }
        17 => {
            let separator = *bytes.get(2).ok_or(AddressError::NotSixOctets)?;
            if separator != b':' && separator != b'-' {
                return Err(AddressError::NotSixOctets);
            }
            for (index, octet) in octets.iter_mut().enumerate() {
                let begin = index.checked_mul(3).ok_or(AddressError::NotSixOctets)?;
                if index != 0
                    && bytes.get(begin.checked_sub(1).ok_or(AddressError::NotSixOctets)?)
                        != Some(&separator)
                {
                    return Err(AddressError::NotSixOctets);
                }
                *octet = parse_hex_pair(bytes.get(begin..begin + 2))?;
            }
        }
        _ => return Err(AddressError::NotSixOctets),
    }
    if octets.iter().all(|octet| *octet == 0) {
        return Err(AddressError::AllZero);
    }
    let mut normalized = String::with_capacity(17);
    for (index, octet) in octets.iter().enumerate() {
        if index != 0 {
            normalized.push(':');
        }
        let high = octet >> 4;
        let low = octet & 0x0f;
        normalized.push(hex_digit(high));
        normalized.push(hex_digit(low));
    }
    Ok(normalized)
}

fn parse_hex_pair(pair: Option<&[u8]>) -> Result<u8, AddressError> {
    let pair = pair.ok_or(AddressError::NotSixOctets)?;
    let high = hex_value(pair.first().copied()).ok_or(AddressError::NotSixOctets)?;
    let low = hex_value(pair.get(1).copied()).ok_or(AddressError::NotSixOctets)?;
    Ok((high << 4) | low)
}

fn hex_value(value: Option<u8>) -> Option<u8> {
    match value? {
        b'0'..=b'9' => value.map(|byte| byte - b'0'),
        b'a'..=b'f' => value.map(|byte| byte - b'a' + 10),
        b'A'..=b'F' => value.map(|byte| byte - b'A' + 10),
        _ => None,
    }
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        _ => char::from(b'A' + (value - 10)),
    }
}

fn is_corebluetooth_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if *byte != b'-' {
                return false;
            }
        } else if hex_value(Some(*byte)).is_none() {
            return false;
        }
    }
    true
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "synthetic configuration fixtures are assertion inputs"
)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn normalizes_all_supported_mac_shapes() {
        for value in ["aa:bb:cc:dd:ee:ff", "AA-BB-CC-DD-EE-FF", "aabbccddeeff"] {
            assert_eq!(
                normalize_linux_target(value, None).expect("MAC"),
                "AA:BB:CC:DD:EE:FF"
            );
        }
    }

    #[test]
    fn opaque_corebluetooth_uuid_requires_explicit_mapping() {
        let uuid = "00112233-4455-6677-8899-aabbccddeeff";
        assert_eq!(
            normalize_linux_target(uuid, None).expect_err("opaque UUID"),
            AddressError::OpaqueCoreBluetoothUuidNeedsMapping
        );
        assert_eq!(
            normalize_linux_target(uuid, Some("11:22:33:44:55:66")).expect("mapping"),
            "11:22:33:44:55:66"
        );
    }

    #[test]
    fn malformed_and_all_zero_addresses_are_rejected() {
        assert_eq!(
            normalize_linux_target("00:00:00:00:00:00", None).expect_err("zero"),
            AddressError::AllZero
        );
        assert_eq!(
            normalize_linux_target("AA:BB:CC:DD:EE", None).expect_err("short"),
            AddressError::NotSixOctets
        );
        assert_eq!(
            normalize_linux_target("AA:BB:CC:DD:EE:FF", Some("bad")).expect_err("bad map"),
            AddressError::NotSixOctets
        );
    }

    #[test]
    fn loads_a_private_bounded_credential_file() {
        let mut file = NamedTempFile::new().expect("temp file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))
                .expect("private mode");
        }
        write!(
            file,
            "{{\"address\":\"aabbccddeeff\",\"linux_address\":null,\"client_id\":\"collector\",\"master_pair_key\":\"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\"}}"
        )
        .expect("JSON");
        let credentials = load_credentials(file.path()).expect("credentials");
        assert_eq!(credentials.target(), "AA:BB:CC:DD:EE:FF");
        assert_eq!(credentials.client_id(), "collector");
    }

    #[test]
    fn rejects_group_or_world_readable_credentials() {
        let mut file = NamedTempFile::new().expect("temp file");
        write!(file, "{{\"address\":\"AA:BB:CC:DD:EE:FF\"}}").expect("JSON");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o644))
                .expect("permissions");
            assert!(matches!(
                load_credentials(file.path()),
                Err(CredentialsError::InsecurePermissions)
            ));
        }
    }
}
