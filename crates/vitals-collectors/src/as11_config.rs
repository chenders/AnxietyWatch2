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

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::{
    as11_crypto::SessionCrypto,
    as11_rpc::{MAX_CLIENT_ID_BYTES, RpcRequest},
};

/// The default operator-managed credential location.
pub const DEFAULT_CREDENTIALS_FILE: &str = ".air11/air11-credentials.json";
/// Maximum credential-file size accepted before JSON parsing begins.
pub const MAX_CREDENTIAL_FILE_BYTES: u64 = 4096;

/// A private credential-file update failure.
#[derive(Debug)]
pub enum CredentialWriteError {
    /// The file or its parent could not be accessed.
    Io(io::Error),
    /// Pairing was asked to replace an existing credential file.
    ExistingPath,
    /// An existing credential file failed validation before a mapping update.
    ExistingCredentials(CredentialsError),
    /// The target address is not a valid six-octet Linux address.
    Address(AddressError),
    /// The client identifier is empty or exceeds its fixed bound.
    ClientIdInvalid,
    /// The supplied master key is zero.
    MasterKeyInvalid,
    /// The bounded credential document could not be serialized.
    Serialization,
}

impl core::fmt::Display for CredentialWriteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "credential_update_io: {error}"),
            Self::ExistingPath => f.write_str("credential_update_existing_path"),
            Self::ExistingCredentials(error) => {
                write!(f, "credential_update_existing_invalid: {error}")
            }
            Self::Address(error) => write!(f, "credential_update_address_{error}"),
            Self::ClientIdInvalid => f.write_str("credential_update_client_id_invalid"),
            Self::MasterKeyInvalid => f.write_str("credential_update_master_key_invalid"),
            Self::Serialization => f.write_str("credential_update_serialization_failed"),
        }
    }
}

impl std::error::Error for CredentialWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::ExistingCredentials(error) => Some(error),
            Self::Address(error) => Some(error),
            _ => None,
        }
    }
}

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

#[derive(Serialize)]
struct PersistedCredentials<'a> {
    address: &'a str,
    linux_address: Option<&'a str>,
    client_id: &'a str,
    master_pair_key: &'a str,
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
    let raw = read_raw_credentials(path)?;
    validate_application_credentials_raw(&raw)?;
    let target = normalize_linux_target(&raw.address, raw.linux_address.as_deref())
        .map_err(CredentialsError::Address)?;

    Ok(Air11Credentials {
        target,
        client_id: raw.client_id.clone(),
        master_pair_key: raw.master_pair_key.clone(),
    })
}

/// Validates reusable application credentials before Linux address discovery.
///
/// This accepts an opaque but well-formed `CoreBluetooth` address because the
/// separate discovery operation is specifically responsible for adding the
/// explicit Linux mapping. It still validates the file, client identifier, and
/// master key before any `BlueZ` operation begins.
///
/// # Errors
/// Returns a named validation or I/O error when the credential file is missing,
/// not private, malformed, or contains invalid application credentials.
pub fn validate_application_credentials(path: &Path) -> Result<(), CredentialsError> {
    let raw = read_raw_credentials(path)?;
    validate_application_credentials_raw(&raw)
}

fn read_raw_credentials(path: &Path) -> Result<RawCredentials, CredentialsError> {
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
    serde_json::from_slice(&bytes).map_err(|_| CredentialsError::JsonInvalid)
}

fn validate_application_credentials_raw(raw: &RawCredentials) -> Result<(), CredentialsError> {
    if raw.address.is_empty() {
        return Err(CredentialsError::Address(AddressError::Empty));
    }
    if let Some(mapping) = raw.linux_address.as_deref() {
        normalize_mac(mapping).map_err(CredentialsError::Address)?;
    } else {
        match normalize_mac(&raw.address) {
            Ok(_) => {}
            Err(AddressError::NotSixOctets) if is_corebluetooth_uuid(&raw.address) => {}
            Err(error) => return Err(CredentialsError::Address(error)),
        }
    }
    if raw.client_id.is_empty() || raw.client_id.len() > MAX_CLIENT_ID_BYTES {
        return Err(CredentialsError::ClientIdInvalid);
    }
    // Exercise the same typed request bound used at the final RPC gate. The
    // request is dropped immediately and its payload is zeroized on drop.
    RpcRequest::request_session(1, &raw.client_id)
        .map_err(|_| CredentialsError::ClientIdInvalid)?;
    SessionCrypto::from_master_key_hex(&raw.master_pair_key)
        .map_err(|_| CredentialsError::MasterKeyInvalid)?;
    Ok(())
}

/// Creates a new private credential file after first-pair onboarding.
///
/// The operation refuses to replace any existing path. The master key is
/// encoded only for the bounded atomic file write and is never returned.
///
/// # Errors
/// Returns a named validation or I/O error when the path already exists, the
/// values are invalid, or the atomic replacement cannot be completed.
pub fn write_new_credentials(
    path: &Path,
    target: &str,
    client_id: &str,
    master_pair_key: &[u8; 32],
) -> Result<(), CredentialWriteError> {
    match fs::symlink_metadata(path) {
        Ok(_) => return Err(CredentialWriteError::ExistingPath),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(CredentialWriteError::Io(error)),
    }
    let target = normalize_linux_target(target, None).map_err(CredentialWriteError::Address)?;
    validate_client_id(client_id)?;
    if master_pair_key.iter().all(|byte| *byte == 0) {
        return Err(CredentialWriteError::MasterKeyInvalid);
    }
    let master_pair_key_hex = Zeroizing::new(crate::as11_crypto::encode_hex(master_pair_key));
    write_document(
        path,
        &target,
        None,
        client_id,
        master_pair_key_hex.as_str(),
        false,
    )
}

/// Adds a normalized Linux mapping to an already valid credential file.
///
/// The existing application credential is read privately and retained; only
/// the address mapping changes in the atomically replaced document.
///
/// # Errors
/// Returns a named validation or I/O error when the existing file is invalid,
/// the address is invalid, or the atomic replacement cannot be completed.
pub fn update_linux_address(path: &Path, target: &str) -> Result<(), CredentialWriteError> {
    let raw = read_raw_credentials(path).map_err(CredentialWriteError::ExistingCredentials)?;
    validate_application_credentials_raw(&raw)
        .map_err(CredentialWriteError::ExistingCredentials)?;
    let normalized = normalize_linux_target(target, None).map_err(CredentialWriteError::Address)?;
    write_document(
        path,
        &raw.address,
        Some(&normalized),
        &raw.client_id,
        &raw.master_pair_key,
        true,
    )
}

fn validate_client_id(client_id: &str) -> Result<(), CredentialWriteError> {
    if client_id.is_empty() || client_id.len() > MAX_CLIENT_ID_BYTES {
        return Err(CredentialWriteError::ClientIdInvalid);
    }
    RpcRequest::request_session(1, client_id)
        .map(|_| ())
        .map_err(|_| CredentialWriteError::ClientIdInvalid)
}

fn write_document(
    path: &Path,
    address: &str,
    linux_address: Option<&str>,
    client_id: &str,
    master_pair_key: &str,
    replace_existing: bool,
) -> Result<(), CredentialWriteError> {
    let document = PersistedCredentials {
        address,
        linux_address,
        client_id,
        master_pair_key,
    };
    let bytes = Zeroizing::new(
        serde_json::to_vec_pretty(&document).map_err(|_| CredentialWriteError::Serialization)?,
    );
    atomic_write_private(path, &bytes, replace_existing)
}

fn atomic_write_private(
    path: &Path,
    bytes: &[u8],
    replace_existing: bool,
) -> Result<(), CredentialWriteError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(CredentialWriteError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                .map_err(CredentialWriteError::Io)?;
        }
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(CredentialWriteError::Serialization)?;
    let mut temporary = None;
    for attempt in 0_u32..16 {
        let candidate = parent.join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&candidate) {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(CredentialWriteError::Io(error)),
        }
    }
    let Some((temporary_path, mut file)) = temporary else {
        return Err(CredentialWriteError::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "credential temporary path unavailable",
        )));
    };
    let result = (|| {
        use io::Write;
        file.write_all(bytes).map_err(CredentialWriteError::Io)?;
        file.sync_all().map_err(CredentialWriteError::Io)?;
        drop(file);
        if replace_existing {
            fs::rename(&temporary_path, path).map_err(CredentialWriteError::Io)?;
        } else {
            // A hard link creates the new path without replacing a path that
            // appeared after the initial existence check. The temporary link
            // is removed only after the destination link is established.
            fs::hard_link(&temporary_path, path).map_err(CredentialWriteError::Io)?;
            fs::remove_file(&temporary_path).map_err(CredentialWriteError::Io)?;
        }
        let directory = fs::File::open(parent).map_err(CredentialWriteError::Io)?;
        directory.sync_all().map_err(CredentialWriteError::Io)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
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
    use std::{fs, io::Write};
    use tempfile::{NamedTempFile, tempdir};

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
            normalize_linux_target(uuid, Some("AA:BB:CC:DD:EE:FF")).expect("mapping"),
            "AA:BB:CC:DD:EE:FF"
        );
    }

    #[test]
    fn malformed_and_all_zero_addresses_are_rejected() {
        let all_zero = concat!("00:00:00:00:", "00:00");
        assert_eq!(
            normalize_linux_target(all_zero, None).expect_err("zero"),
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
    fn writes_private_credentials_once_and_refuses_replacement() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("air11-credentials.json");
        let master_key = [1_u8; 32];
        write_new_credentials(&path, "aa:bb:cc:dd:ee:ff", "collector", &master_key)
            .expect("write credentials");
        let credentials = load_credentials(&path).expect("load credentials");
        assert_eq!(credentials.target(), "AA:BB:CC:DD:EE:FF");
        assert_eq!(credentials.client_id(), "collector");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(matches!(
            write_new_credentials(&path, "AA:BB:CC:DD:EE:FF", "other", &master_key),
            Err(CredentialWriteError::ExistingPath)
        ));
    }

    #[test]
    fn mapping_update_preserves_the_existing_application_credential() {
        let mut file = NamedTempFile::new().expect("temporary file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))
                .expect("private mode");
        }
        write!(
            file,
            "{{\"address\":\"00112233-4455-6677-8899-aabbccddeeff\",\"linux_address\":null,\"client_id\":\"collector\",\"master_pair_key\":\"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\"}}"
        )
        .expect("JSON");
        validate_application_credentials(file.path()).expect("application credentials");
        let mapped_address = concat!("11", "-", "22", "-", "33", "-", "44", "-", "55", "-", "66");
        update_linux_address(file.path(), mapped_address).expect("mapping update");
        let credentials = load_credentials(file.path()).expect("updated credentials");
        let expected_address = concat!("11", ":", "22", ":", "33", ":", "44", ":", "55", ":", "66");
        assert_eq!(credentials.target(), expected_address);
        assert_eq!(credentials.client_id(), "collector");
        let document = fs::read_to_string(file.path()).expect("updated JSON");
        assert!(document.contains("00112233-4455-6677-8899-aabbccddeeff"));
        assert!(document.contains(expected_address));
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
