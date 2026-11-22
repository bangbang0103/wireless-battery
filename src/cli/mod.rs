//! CLI command definitions and top-level dispatch.

mod output;
mod probe;

use clap::{Parser, Subcommand};
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
    },
    /// Show passive provider, cache and permission diagnostics.
    Diagnose {
        /// Device ID obtained from `list`.
        device_id: String,
        /// Emit diagnostics as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Probe or listen to raw HID reports on any HID endpoint for reverse engineering.
    Probe {
        /// Device ID obtained from `list`.
        device_id: String,
        /// Report ID for --send / --send-feature (decimal or 0x.. hex).
        #[arg(long, default_value = "0", value_parser = probe::parse_u8_hex_or_dec)]
        report_id: u8,
        /// Send an Output report payload (hex bytes, repeatable).
        #[arg(long = "send", value_name = "HEX")]
        send_outputs: Vec<String>,
        /// Use HID SET_REPORT (control pipe) instead of write_output for --send.
        #[arg(long)]
        set_output: bool,
        /// Send a Feature report payload (hex bytes, repeatable).
        #[arg(long = "send-feature", value_name = "HEX")]
        send_features: Vec<String>,
        /// Read a Feature report for the given report ID (decimal or 0x.. hex, repeatable).
        #[arg(long = "get-feature", value_name = "ID", value_parser = probe::parse_u8_hex_or_dec)]
        get_features: Vec<u8>,
        /// Read an Input report via control pipe GET_REPORT (decimal or 0x.. hex, repeatable).
        #[arg(long = "get-input", value_name = "ID", value_parser = probe::parse_u8_hex_or_dec)]
        get_inputs: Vec<u8>,
        /// Expected payload length for --get-feature / --get-input.
        #[arg(long, default_value_t = 64)]
        length: usize,
        /// Passively listen for incoming Input reports for N milliseconds (defaults to 2000ms if no send/get options are given).
        #[arg(long, value_name = "MS")]
        listen: Option<u64>,
        /// Maximum Input reply reports to read after each --send.
        #[arg(long, default_value_t = 4)]
        read_replies: usize,
        /// Reply read timeout in milliseconds after each --send.
        #[arg(long, default_value_t = 500)]
        timeout_ms: u64,
        /// Emit structured JSON on stdout.
        #[arg(long)]
        json: bool,
    },
}

impl Command {
    const fn wants_json(&self) -> bool {
        match self {
            Self::List { json, .. }
            | Self::Read { json, .. }
            | Self::Diagnose { json, .. }
            | Self::Probe { json, .. } => *json,
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
            read,
            timeout_ms,
        } => {
            let client = client(true, true)?;
            let devices = client.list_devices()?;
            if read {
                let entries = output::read_device_list(&client, devices, timeout_ms);
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
        } => {
            if allow_stale {
                eprintln!(
                    "warning: --allow-stale has no effect in one-shot CLI invocations because cache is process-local"
                );
            }
            let client = client(true, true)?;
            let reading = client.read_battery_with_options(
                &device_id,
                ReadOptions {
                    timeout: Duration::from_millis(timeout_ms),
                    allow_stale,
                    allow_experimental: true,
                },
            )?;
            if json {
                output::print_json(&reading)?;
            } else {
                output::print_reading(&reading);
            }
        }
        Command::Diagnose { device_id, json } => {
            let diagnostic = client(true, true)?.diagnose(&device_id)?;
            if json {
                output::print_json(&diagnostic)?;
            } else {
                output::print_json_pretty(&diagnostic)?;
            }
        }
        Command::Probe {
            device_id,
            report_id,
            send_outputs,
            set_output,
            send_features,
            get_features,
            get_inputs,
            length,
            listen,
            read_replies,
            timeout_ms,
            json,
        } => probe::run(probe::ProbeOptions {
            device_id,
            report_id,
            send_outputs,
            set_output,
            send_features,
            get_features,
            get_inputs,
            length,
            listen_ms: listen,
            read_replies,
            timeout_ms,
            json,
        })?,
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
