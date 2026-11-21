//! Sanitized real-device HID capture into JSON fixture format.

use super::validate::{reject_sensitive_fields, validate_fixture_schema};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use wireless_battery::model::{BatteryReading, ClientConfig, Device};
use wireless_battery::registry::Registry;
use wireless_battery::transport::record::RecordTransport;
use wireless_battery::transport::replay::{
    Expectation, Fixture, FixtureEnvironment, FixtureSource, FixtureTarget,
};
use wireless_battery::transport::HidApiTransport;
use wireless_battery::{BatteryClient, BatteryError};

pub(super) fn run(
    device_id: &str,
    output: &Path,
    allow_experimental: bool,
    timeout_ms: u64,
) -> Result<(), BatteryError> {
    let hid = Arc::new(
        HidApiTransport::new().map_err(|error| BatteryError::Transport {
            message: error.to_string(),
        })?,
    );
    let recorder = Arc::new(RecordTransport::new(hid));
    let client = BatteryClient::with_transport(
        ClientConfig {
            allow_experimental,
            default_timeout: Duration::from_millis(timeout_ms),
            ..ClientConfig::default()
        },
        recorder.clone(),
        Registry::builtin()?,
    )?;
    let device = find_device(&client, device_id)?;
    let reading = client.read_battery(device_id)?;
    let target_kind = if device
        .provider_id
        .as_deref()
        .and_then(|id| client.registry().find(id))
        .is_some_and(|provider| provider.is_declarative())
    {
        "package"
    } else {
        "provider"
    };
    let fixture = Fixture {
        schema_version: 1,
        target: FixtureTarget {
            kind: target_kind.to_owned(),
            id: device.provider_id.clone().unwrap_or_default(),
        },
        case: "captured_reading".to_owned(),
        source: FixtureSource::Real,
        environment: FixtureEnvironment {
            os: Some(std::env::consts::OS.to_owned()),
            connection: "usb_hid".to_owned(),
            firmware: None,
        },
        endpoint: device.endpoint.clone(),
        steps: recorder
            .steps()
            .map_err(|message| BatteryError::Internal { message })?,
        expect: expectation(&reading),
    };
    let value = serde_json::to_value(&fixture).map_err(|error| BatteryError::Internal {
        message: error.to_string(),
    })?;
    validate_fixture_schema(&value, &output.display().to_string())?;
    reject_sensitive_fields(&value)?;
    let text = serde_json::to_vec_pretty(&fixture).map_err(|error| BatteryError::Internal {
        message: error.to_string(),
    })?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| BatteryError::InvalidFixture {
            fixture: output.display().to_string(),
            reason: format!("cannot create output without overwriting: {error}"),
        })?;
    file.write_all(&text)
        .map_err(|error| BatteryError::InvalidFixture {
            fixture: output.display().to_string(),
            reason: error.to_string(),
        })?;
    eprintln!(
        "captured {} for {}: {} step(s) -> {}",
        device.name,
        device.id,
        fixture.steps.len(),
        output.display()
    );
    Ok(())
}

fn find_device(client: &BatteryClient, id: &str) -> Result<Device, BatteryError> {
    client
        .list_devices()?
        .into_iter()
        .find(|device| device.id.0 == id)
        .ok_or_else(|| BatteryError::NotFound {
            device_id: id.to_owned(),
        })
}

fn expectation(reading: &BatteryReading) -> Expectation {
    Expectation::Reading {
        percentage: reading.percentage,
        level: reading.level,
        voltage_mv: reading.voltage_mv,
        charge_state: reading.charge_state,
        reachability: reading.reachability,
        precision: reading.precision,
    }
}
