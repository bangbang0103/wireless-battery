//! Interactive HID probing and passive listening for reverse-engineering new devices.

use serde::Serialize;
use std::time::{Duration, Instant};
use wireless_battery::codec::{decode_hex, encode_hex};
use wireless_battery::model::{ClientConfig, Device};
use wireless_battery::registry::Registry;
use wireless_battery::transport::{HidApiTransport, HidSession, Transport, TransportError};
use wireless_battery::{BatteryClient, BatteryError};

pub(super) struct ProbeOptions {
    pub device_id: String,
    pub report_id: u8,
    pub send_outputs: Vec<String>,
    pub set_output: bool,
    pub send_features: Vec<String>,
    pub get_features: Vec<u8>,
    pub get_inputs: Vec<u8>,
    pub length: usize,
    pub listen_ms: Option<u64>,
    pub read_replies: usize,
    pub timeout_ms: u64,
    pub json: bool,
}

#[derive(Serialize)]
struct ProbeReport {
    device: Device,
    events: Vec<ProbeEvent>,
}

#[derive(Serialize)]
struct ProbeEvent {
    elapsed_ms: u64,
    kind: String,
    report_id: u8,
    len: usize,
    hex: String,
}

pub(super) fn run(options: ProbeOptions) -> Result<(), BatteryError> {
    let hid =
        std::sync::Arc::new(
            HidApiTransport::new().map_err(|error| BatteryError::Transport {
                message: error.to_string(),
            })?,
        );
    let client =
        BatteryClient::with_transport(ClientConfig::default(), hid.clone(), Registry::builtin()?)?;
    let device = client
        .list_devices()?
        .into_iter()
        .find(|d| d.id.0 == options.device_id)
        .ok_or_else(|| BatteryError::NotFound {
            device_id: options.device_id.clone(),
        })?;

    let mut session = hid
        .open(&device.endpoint_id)
        .map_err(|error| error.into_battery(&device.endpoint_id))?;

    let started = Instant::now();
    let mut events = Vec::new();

    for hex_str in &options.send_features {
        let payload = parse_hex_arg(hex_str)?;
        session
            .send_feature(options.report_id, &payload)
            .map_err(|error| error.into_battery(&device.endpoint_id))?;
        record_event(
            &mut events,
            started,
            "send_feature",
            options.report_id,
            &payload,
            options.json,
        );
    }

    for &report_id in &options.get_features {
        let data = session
            .get_feature(report_id, options.length)
            .map_err(|error| error.into_battery(&device.endpoint_id))?;
        record_event(
            &mut events,
            started,
            "get_feature",
            report_id,
            &data,
            options.json,
        );
    }

    for &report_id in &options.get_inputs {
        let data = session
            .get_input_report(report_id, options.length)
            .map_err(|error| error.into_battery(&device.endpoint_id))?;
        record_event(
            &mut events,
            started,
            "get_input",
            report_id,
            &data,
            options.json,
        );
    }

    for hex_str in &options.send_outputs {
        let payload = parse_hex_arg(hex_str)?;
        let kind = if options.set_output {
            session
                .set_output(options.report_id, &payload)
                .map_err(|error| error.into_battery(&device.endpoint_id))?;
            "set_output"
        } else {
            session
                .write_output(options.report_id, &payload)
                .map_err(|error| error.into_battery(&device.endpoint_id))?;
            "write_output"
        };
        record_event(
            &mut events,
            started,
            kind,
            options.report_id,
            &payload,
            options.json,
        );
        read_replies(
            session.as_mut(),
            &device,
            started,
            options.read_replies,
            Duration::from_millis(options.timeout_ms),
            &mut events,
            options.json,
        )?;
    }

    let has_active_ops = !options.send_outputs.is_empty()
        || !options.send_features.is_empty()
        || !options.get_features.is_empty()
        || !options.get_inputs.is_empty();
    let listen_ms = options
        .listen_ms
        .or_else(|| (!has_active_ops).then_some(2_000));

    if let Some(ms) = listen_ms {
        listen_loop(
            session.as_mut(),
            &device,
            started,
            Duration::from_millis(ms),
            &mut events,
            options.json,
        )?;
    }

    if options.json {
        super::output::print_json(&ProbeReport { device, events })?;
    } else if events.is_empty() {
        eprintln!("no HID reports observed within timeout");
    }
    Ok(())
}

fn read_replies(
    session: &mut dyn HidSession,
    device: &Device,
    started: Instant,
    max_replies: usize,
    budget: Duration,
    events: &mut Vec<ProbeEvent>,
    json: bool,
) -> Result<(), BatteryError> {
    let deadline = Instant::now() + budget;
    for _ in 0..max_replies {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        match session.read_input(remaining) {
            Ok(report) => {
                record_event(
                    events,
                    started,
                    "read_input",
                    report.report_id,
                    &report.data,
                    json,
                );
            }
            Err(TransportError::Timeout) => break,
            Err(error) => return Err(error.into_battery(&device.endpoint_id)),
        }
    }
    Ok(())
}

fn listen_loop(
    session: &mut dyn HidSession,
    device: &Device,
    started: Instant,
    duration: Duration,
    events: &mut Vec<ProbeEvent>,
    json: bool,
) -> Result<(), BatteryError> {
    let deadline = Instant::now() + duration;
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        let step = remaining.min(Duration::from_millis(200));
        match session.read_input(step) {
            Ok(report) => {
                record_event(
                    events,
                    started,
                    "read_input",
                    report.report_id,
                    &report.data,
                    json,
                );
            }
            Err(TransportError::Timeout) => continue,
            Err(error) => return Err(error.into_battery(&device.endpoint_id)),
        }
    }
    Ok(())
}

fn record_event(
    events: &mut Vec<ProbeEvent>,
    started: Instant,
    kind: &str,
    report_id: u8,
    bytes: &[u8],
    json: bool,
) {
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let hex = encode_hex(bytes);
    if !json {
        println!(
            "[{elapsed_ms:>5}ms] {kind:<12} report_id=0x{report_id:02X} len={} hex={hex}",
            bytes.len()
        );
    }
    events.push(ProbeEvent {
        elapsed_ms,
        kind: kind.to_owned(),
        report_id,
        len: bytes.len(),
        hex,
    });
}

fn parse_hex_arg(value: &str) -> Result<Vec<u8>, BatteryError> {
    decode_hex(value).map_err(|message| BatteryError::Internal { message })
}

pub(super) fn parse_u8_hex_or_dec(value: &str) -> Result<u8, String> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).map_err(|err| format!("invalid hex byte `{value}`: {err}"))
    } else {
        trimmed
            .parse::<u8>()
            .map_err(|err| format!("invalid byte `{value}`: {err}"))
    }
}
