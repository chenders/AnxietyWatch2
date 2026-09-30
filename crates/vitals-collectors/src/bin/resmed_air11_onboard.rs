//! Operator-only `AirSense` 11 discovery and first application pairing.
//!
//! This binary is deliberately separate from `resmed-air11-collector`. It
//! never starts a service and exposes no arbitrary RPC or credential argument.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("resmed-air11-onboard unavailable: this command requires Linux BlueZ");
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::Command,
    time::Duration,
};

#[cfg(target_os = "linux")]
use getrandom::fill;
#[cfg(target_os = "linux")]
use zeroize::{Zeroize, Zeroizing};

#[cfg(target_os = "linux")]
use vitals_collectors::{
    as11_config::{self, CredentialsError},
    as11_crypto::{SessionCrypto, SrpClient},
    as11_onboarding::{
        Air11PairingLink, DEFAULT_DISCOVERY_SECONDS, MAX_DISCOVERY_SECONDS, MIN_DISCOVERY_SECONDS,
        OnboardingError,
    },
};

#[cfg(target_os = "linux")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    match run().await {
        Ok(()) => {}
        Err(error) => {
            eprintln!("resmed-air11-onboard: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
enum Mode {
    Discover,
    Pair,
}

#[cfg(target_os = "linux")]
impl Mode {
    const fn name(self) -> &'static str {
        match self {
            Self::Discover => "DISCOVER",
            Self::Pair => "PAIR",
        }
    }
}

#[cfg(target_os = "linux")]
struct Options {
    mode: Mode,
    adapter: String,
    credentials: PathBuf,
    duration: Duration,
    selected_address: Option<String>,
}

#[cfg(target_os = "linux")]
struct PairingMaterial {
    client_id: Zeroizing<String>,
    master_key: Zeroizing<[u8; 32]>,
}

#[cfg(target_os = "linux")]
async fn run() -> Result<(), OnboardingError> {
    let options = parse_options()?;
    authorize(&options)?;
    match options.mode {
        Mode::Discover => run_discovery(&options).await,
        Mode::Pair => run_pairing(&options).await,
    }
}

#[cfg(target_os = "linux")]
fn parse_options() -> Result<Options, OnboardingError> {
    let mut args = std::env::args().skip(1);
    let mut mode = None;
    let mut adapter = String::from("hci0");
    let mut credentials = as11_config::default_credentials_path()?;
    let mut duration = DEFAULT_DISCOVERY_SECONDS;
    let mut selected_address = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "discover" if mode.is_none() => mode = Some(Mode::Discover),
            "pair" if mode.is_none() => mode = Some(Mode::Pair),
            "--adapter" => {
                adapter = next_option_value(&mut args, "adapter")?;
            }
            "--credentials" => {
                credentials = PathBuf::from(next_option_value(&mut args, "credentials")?);
            }
            "--duration" => {
                let value = next_option_value(&mut args, "duration")?;
                duration = value
                    .parse::<u64>()
                    .map_err(|_| OnboardingError::PairingProtocol {
                        reason: "duration_integer",
                    })?;
            }
            "--address" => {
                selected_address = Some(next_option_value(&mut args, "address")?);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                return Err(OnboardingError::PairingProtocol {
                    reason: "command_arguments",
                });
            }
        }
    }

    let mode = mode.ok_or(OnboardingError::PairingProtocol {
        reason: "mode_required",
    })?;
    if adapter.is_empty() {
        return Err(OnboardingError::PairingProtocol {
            reason: "adapter_empty",
        });
    }
    if !(MIN_DISCOVERY_SECONDS..=MAX_DISCOVERY_SECONDS).contains(&duration) {
        return Err(OnboardingError::InvalidDuration {
            seconds: duration,
            min: MIN_DISCOVERY_SECONDS,
            max: MAX_DISCOVERY_SECONDS,
        });
    }
    Ok(Options {
        mode,
        adapter,
        credentials,
        duration: Duration::from_secs(duration),
        selected_address,
    })
}

#[cfg(target_os = "linux")]
fn next_option_value(
    args: &mut impl Iterator<Item = String>,
    _name: &'static str,
) -> Result<String, OnboardingError> {
    args.next()
        .filter(|value| !value.is_empty())
        .ok_or(OnboardingError::PairingProtocol {
            reason: "option_value_missing",
        })
}

#[cfg(target_os = "linux")]
fn print_usage() {
    println!(
        "Usage: resmed-air11-onboard <discover|pair> [--adapter hci0] [--duration seconds] [--address MAC] [--credentials path]"
    );
    println!("discover requires valid saved credentials; pair requires an absent credential file.");
    println!(
        "The operation is finite, FD56-filtered, and requires a matching interactive confirmation."
    );
}

#[cfg(target_os = "linux")]
fn authorize(options: &Options) -> Result<(), OnboardingError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(OnboardingError::PairingProtocol {
            reason: "interactive_confirmation_required",
        });
    }
    let seconds = options.duration.as_secs();
    let token = format!("AIR11-ONBOARD-{}-{seconds}", options.mode.name());
    eprintln!(
        "This will run a bounded AirSense {} operation for {seconds} seconds on the selected adapter.",
        options.mode.name().to_ascii_lowercase()
    );
    eprintln!("It will not start the collector or send therapy/control RPCs.");
    eprint!("Type {token} to authorize: ");
    io::stderr()
        .flush()
        .map_err(|_| OnboardingError::PairingProtocol {
            reason: "confirmation_output",
        })?;
    let mut confirmation = String::new();
    io::stdin()
        .read_line(&mut confirmation)
        .map_err(|_| OnboardingError::PairingProtocol {
            reason: "confirmation_input",
        })?;
    if confirmation.trim_end() != token {
        return Err(OnboardingError::PairingProtocol {
            reason: "confirmation_mismatch",
        });
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn run_discovery(options: &Options) -> Result<(), OnboardingError> {
    as11_config::validate_application_credentials(&options.credentials)?;
    let target = vitals_collectors::as11_onboarding::discover_target(
        &options.adapter,
        options.duration,
        options.selected_address.as_deref(),
    )
    .await?;
    as11_config::update_linux_address(&options.credentials, &target)?;
    println!("AirSense discovery complete: private Linux mapping updated");
    Ok(())
}

#[cfg(target_os = "linux")]
async fn run_pairing(options: &Options) -> Result<(), OnboardingError> {
    match as11_config::load_credentials(&options.credentials) {
        Ok(_) => {
            return Err(OnboardingError::PairingProtocol {
                reason: "valid_credentials_already_exist; use_discover",
            });
        }
        Err(CredentialsError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(OnboardingError::PairingProtocol {
                reason: "credential_path_exists_or_is_invalid; refusing_replace",
            });
        }
    }
    let target = vitals_collectors::as11_onboarding::discover_target(
        &options.adapter,
        options.duration,
        options.selected_address.as_deref(),
    )
    .await?;
    let mut link = Air11PairingLink::connect(&options.adapter, &target).await?;
    let pairing_result = pair_over_link(&mut link).await;
    let disconnect_result = link.disconnect().await;
    let material = match pairing_result {
        Ok(material) => material,
        Err(error) => {
            if disconnect_result.is_err() {
                return Err(OnboardingError::CleanupFailed {
                    operation: "gatt_disconnect",
                });
            }
            return Err(error);
        }
    };
    disconnect_result?;
    as11_config::write_new_credentials(
        &options.credentials,
        &target,
        &material.client_id,
        &material.master_key,
    )?;
    println!("AirSense first pairing complete: private credentials written");
    Ok(())
}

#[cfg(target_os = "linux")]
async fn pair_over_link(link: &mut Air11PairingLink) -> Result<PairingMaterial, OnboardingError> {
    let mut private_key = Zeroizing::new([0_u8; 32]);
    fill(private_key.as_mut()).map_err(|_| OnboardingError::PairingProtocol {
        reason: "random_private_key_unavailable",
    })?;
    let srp = SrpClient::begin(*private_key)?;
    let public_key = srp.public_key_hex();
    let start = link.start_key_exchange(&public_key).await?;
    let passkey = read_passkey()?;
    let proof = srp.finish(passkey.as_bytes(), start.server_public_key(), start.salt())?;
    let confirmation = link.confirm_key_exchange(proof.client_proof()).await?;
    if !proof.verify_server_hex(confirmation.server_confirmation()) {
        return Err(OnboardingError::PairingProtocol {
            reason: "server_confirmation_invalid",
        });
    }
    let master_key = Zeroizing::new(*proof.master_key());
    let mut session = SessionCrypto::new(*master_key)?;
    session.derive_session_key(confirmation.nonce())?;
    Ok(PairingMaterial {
        client_id: Zeroizing::new(confirmation.client_id().to_owned()),
        master_key,
    })
}

#[cfg(target_os = "linux")]
struct EchoGuard {
    active: bool,
}

#[cfg(target_os = "linux")]
impl EchoGuard {
    fn disable() -> Result<Self, OnboardingError> {
        set_echo(false)?;
        Ok(Self { active: true })
    }

    fn restore(&mut self) -> Result<(), OnboardingError> {
        if self.active {
            set_echo(true)?;
            self.active = false;
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for EchoGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = set_echo(true);
        }
    }
}

#[cfg(target_os = "linux")]
fn set_echo(enabled: bool) -> Result<(), OnboardingError> {
    let argument = if enabled { "echo" } else { "-echo" };
    let status = Command::new("stty").arg(argument).status().map_err(|_| {
        OnboardingError::PairingProtocol {
            reason: "terminal_echo_control",
        }
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(OnboardingError::PairingProtocol {
            reason: "terminal_echo_control",
        })
    }
}

#[cfg(target_os = "linux")]
fn read_passkey() -> Result<Zeroizing<String>, OnboardingError> {
    eprint!("Enter the four-digit passkey shown on the AirSense screen: ");
    io::stderr()
        .flush()
        .map_err(|_| OnboardingError::PairingProtocol {
            reason: "passkey_prompt",
        })?;
    let mut echo = EchoGuard::disable()?;
    let mut input = String::new();
    let read_result = io::stdin().read_line(&mut input);
    let restore_result = echo.restore();
    eprintln!();
    read_result.map_err(|_| OnboardingError::PairingProtocol {
        reason: "passkey_input",
    })?;
    restore_result?;
    let passkey = Zeroizing::new(input.trim().to_owned());
    input.zeroize();
    if passkey.len() != 4 || !passkey.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(OnboardingError::PairingProtocol {
            reason: "passkey_shape",
        });
    }
    Ok(passkey)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn mode_names_are_fixed() {
        assert_eq!(Mode::Discover.name(), "DISCOVER");
        assert_eq!(Mode::Pair.name(), "PAIR");
    }
}
