//! CLI command definitions and top-level dispatch.

mod capture;
mod output;
mod replay;
mod validate;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;
use wireless_battery::model::{ClientConfig, ReadOptions};
use wireless_battery::{BatteryClient, BatteryError};

#[derive(Parser)]
#[command(
    name = "wireless-battery",
    version,
    about = "Read peripheral battery levels conservatively"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Enumerate all HID and known BLE endpoints without device I/O.
    List {
        /// Emit a stable JSON array on stdout.
        #[arg(long)]
        json: bool,
        /// Mark experimental providers as readable.
        #[arg(long)]
        include_experimental: bool,
        /// Explicitly read every currently readable device after passive enumeration.
        #[arg(long)]
        read: bool,
        /// Total timeout for each device when --read is used.
        #[arg(long, default_value_t = 5_000)]
        timeout_ms: u64,
    },
    /// Read one session-local device ID.
    Read {
        /// Device ID obtained from `list` in the same process integration session.
        device_id: String,
        /// Emit the complete reading as JSON.
        #[arg(long)]
        json: bool,
        /// Total read budget in milliseconds.
        #[arg(long, default_value_t = 1_000)]
        timeout_ms: u64,
        /// Return the last successful value after a refresh error (library cache only; one-shot CLI starts with an empty cache).
        #[arg(long)]
        allow_stale: bool,
        /// Allow experimental provider I/O.
        #[arg(long)]
        include_experimental: bool,
    },
    /// Show passive provider, cache and permission diagnostics.
    Diagnose {
        /// Device ID obtained from `list`.
        device_id: String,
        /// Emit diagnostics as JSON.
        #[arg(long)]
        json: bool,
        /// Mark experimental providers as enabled during diagnostics.
        #[arg(long)]
        include_experimental: bool,
    },
    /// Capture one registered HID provider query as a sanitized real fixture.
    Capture {
        /// Device ID obtained from `list`.
        #[arg(long)]
        device: String,
        /// New fixture path; existing files are never overwritten.
        #[arg(long)]
        output: PathBuf,
        /// Allow experimental provider I/O.
        #[arg(long)]
        include_experimental: bool,
        /// Total capture budget in milliseconds.
        #[arg(long, default_value_t = 5_000)]
        timeout_ms: u64,
    },
    /// Replay one or more JSON sessions without hardware.
    Replay {
        /// Fixture paths to replay in order.
        #[arg(long = "fixture", required = true, num_args = 1)]
        fixtures: Vec<PathBuf>,
    },
    /// Validate embedded and on-disk packages, fixtures, references and privacy rules.
    Validate {
        /// Additional package directory; defaults to packages/vendors when present.
        #[arg(long)]
        packages: Option<PathBuf>,
        /// Fixture directory; defaults to fixtures when present.
        #[arg(long)]
        fixtures: Option<PathBuf>,
    },
}

impl Command {
    const fn wants_json(&self) -> bool {
        match self {
            Self::List { json, .. } | Self::Read { json, .. } | Self::Diagnose { json, .. } => {
                *json
            }
            _ => false,
        }
    }
}

pub(crate) fn run() -> i32 {
    let cli = Cli::parse();
    let json = cli.command.wants_json();
    match execute(cli.command) {
        Ok(()) => 0,
        Err(error) => {
            if json {
                let value = serde_json::json!({
                    "error": { "kind": error.kind(), "message": error.to_string() }
                });
                println!(
                    "{}",
                    serde_json::to_string(&value).unwrap_or_else(|_| {
                        "{\"error\":{\"kind\":\"internal\",\"message\":\"JSON serialization failed\"}}".to_owned()
                    })
                );
            } else {
                eprintln!("error: {error}");
            }
            error.exit_code()
        }
    }
}

fn execute(command: Command) -> Result<(), BatteryError> {
    match command {
        Command::List {
            json,
            include_experimental,
            read,
            timeout_ms,
        } => {
            let client = client(include_experimental, true)?;
            let devices = client.list_devices()?;
            if read {
                let entries =
                    output::read_device_list(&client, devices, timeout_ms, include_experimental);
                if json {
                    output::print_json(&entries)?;
                } else {
                    for entry in entries {
                        output::print_list_entry(entry);
                    }
                }
            } else if json {
                output::print_json(&devices)?;
            } else {
                for device in devices {
                    println!(
                        "{}\t{}\t{:?}\t{:?}\t{}\tnot read\tunknown",
                        device.id,
                        device.name,
                        device.match_status,
                        device.support_status,
                        device.provider_id.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }
        Command::Read {
            device_id,
            json,
            timeout_ms,
            allow_stale,
            include_experimental,
        } => {
            if allow_stale {
                eprintln!(
                    "warning: --allow-stale has no effect in one-shot CLI invocations because cache is process-local"
                );
            }
            let client = client(include_experimental, true)?;
            let reading = client.read_battery_with_options(
                &device_id,
                ReadOptions {
                    timeout: Duration::from_millis(timeout_ms),
                    allow_stale,
                    allow_experimental: include_experimental,
                },
            )?;
            if json {
                output::print_json(&reading)?;
            } else {
                output::print_reading(&reading);
            }
        }
        Command::Diagnose {
            device_id,
            json,
            include_experimental,
        } => {
            let diagnostic = client(include_experimental, true)?.diagnose(&device_id)?;
            if json {
                output::print_json(&diagnostic)?;
            } else {
                output::print_json_pretty(&diagnostic)?;
            }
        }
        Command::Capture {
            device,
            output: output_path,
            include_experimental,
            timeout_ms,
        } => capture::run(&device, &output_path, include_experimental, timeout_ms)?,
        Command::Replay { fixtures } => {
            for path in fixtures {
                replay::run(&path)?;
            }
        }
        Command::Validate { packages, fixtures } => validate::run(packages, fixtures)?,
    }
    Ok(())
}

fn client(allow_experimental: bool, cache: bool) -> Result<BatteryClient, BatteryError> {
    BatteryClient::new(ClientConfig {
        allow_experimental,
        enable_cache: cache,
        ..ClientConfig::default()
    })
}
