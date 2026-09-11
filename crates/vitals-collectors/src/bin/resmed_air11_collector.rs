//! Read-only Linux `AirSense` 11 stream collector.
//!
//! The portable configuration, identity, and sample policy live in the
//! library.  This binary only loads the operator-managed file, uses the
//! bounded `BlueZ` transport, and exercises the stream builder.  Appending to
//! `vitalsd` and journaling are added by the following task; this task does not
//! create a second history path.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("resmed-air11-collector unavailable: this binary requires Linux BlueZ; no host mock");
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("resmed-air11-collector: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
struct Options {
    credentials_path: std::path::PathBuf,
    adapter: String,
    state_dir: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_options()?;
    let credentials = vitals_collectors::as11_config::load_credentials(&options.credentials_path)?;
    let mut transport =
        vitals_collectors::as11_bluez::Air11BleTransport::connect(&credentials, &options.adapter)
            .await?;
    transport.authenticate(credentials.client_id()).await?;

    let mut session_minter = vitals_core::SessionMinter::open(&options.state_dir)?;
    let session_id = session_minter.mint()?;
    let mut builder = vitals_collectors::as11_build::Air11SampleBuilder::new(session_id)?;
    transport.start_stream().await?;

    let mut shutdown = Box::pin(tokio::signal::ctrl_c());
    loop {
        tokio::select! {
            frame = transport.next_stream_frame() => {
                let frame = frame?;
                let batch = builder.build_frame(&frame, std::time::SystemTime::now())?;
                // T-A11-4 replaces this sink with the journal + vitalsd append
                // path.  Keeping the bounded builder in this loop ensures the
                // transport exercises source identity without inventing a
                // parallel history store here.
                let _ = batch.samples.len();
            }
            signal = &mut shutdown => {
                signal?;
                break;
            }
        }
    }
    transport.disconnect().await?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn parse_options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut credentials_path = vitals_collectors::as11_config::default_credentials_path()?;
    let mut adapter = String::from("hci0");
    let mut state_dir = std::path::PathBuf::from("/var/lib/vitals/air11");
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        let argument = argument
            .into_string()
            .map_err(|_| "command-line option is not UTF-8")?;
        match argument.as_str() {
            "--credentials" => {
                let path = args.next().ok_or("--credentials requires a path")?;
                credentials_path = std::path::PathBuf::from(path);
            }
            "--adapter" => {
                let name = args.next().ok_or("--adapter requires a name")?;
                adapter = name
                    .into_string()
                    .map_err(|_| "--adapter name is not UTF-8")?;
                if adapter.is_empty() {
                    return Err("--adapter cannot be empty".into());
                }
            }
            "--state-dir" => {
                let path = args.next().ok_or("--state-dir requires a path")?;
                state_dir = std::path::PathBuf::from(path);
            }
            "--help" => {
                println!(
                    "usage: resmed-air11-collector [--credentials PATH] [--adapter NAME] [--state-dir PATH]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option={argument}").into()),
        }
    }
    Ok(Options {
        credentials_path,
        adapter,
        state_dir,
    })
}
