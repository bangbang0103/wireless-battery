//! Text and JSON formatting for CLI device lists and readings.

use std::io::Write;
use std::time::Duration;
use wireless_battery::model::{BatteryReading, Device, ReadOptions};
use wireless_battery::{BatteryClient, BatteryError};

#[derive(serde::Serialize)]
pub(super) struct ListEntry {
    #[serde(flatten)]
    pub(super) device: Device,
    pub(super) reading: Option<BatteryReading>,
    pub(super) read_error: Option<ListReadError>,
}

#[derive(serde::Serialize)]
pub(super) struct ListReadError {
    pub(super) kind: wireless_battery::ErrorKind,
    pub(super) message: String,
}

pub(super) fn read_device_list(
    client: &BatteryClient,
    devices: Vec<Device>,
    timeout_ms: u64,
) -> Vec<ListEntry> {
    devices
        .into_iter()
        .map(|device| {
            if !device.readable {
                return ListEntry {
                    device,
                    reading: None,
                    read_error: None,
                };
            }
            match client.read_battery_with_options(
                &device.id.0,
                ReadOptions {
                    timeout: Duration::from_millis(timeout_ms),
                    allow_stale: false,
                    allow_experimental: true,
                },
            ) {
                Ok(reading) => ListEntry {
                    device,
                    reading: Some(reading),
                    read_error: None,
                },
                Err(error) => ListEntry {
                    device,
                    reading: None,
                    read_error: Some(ListReadError {
                        kind: error.kind(),
                        message: error.to_string(),
                    }),
                },
            }
        })
        .collect()
}

pub(super) fn format_reading_value(reading: &BatteryReading) -> String {
    if let Some(percentage) = reading.percentage {
        format!("{percentage}%")
    } else if let Some(level) = reading.level {
        format!("{level:?}").to_lowercase()
    } else if let Some(voltage_mv) = reading.voltage_mv {
        format!("{voltage_mv}mV")
    } else {
        "unknown".to_owned()
    }
}

pub(super) fn print_list_entry(entry: ListEntry) {
    let (battery, charge) = entry.reading.as_ref().map_or_else(
        || {
            if entry.read_error.is_some() {
                ("read error".to_owned(), "unknown".to_owned())
            } else {
                ("not read".to_owned(), "unknown".to_owned())
            }
        },
        |reading| {
            (
                format_reading_value(reading),
                format!("{:?}", reading.charge_state).to_lowercase(),
            )
        },
    );
    if let Some(error) = &entry.read_error {
        eprintln!("warning: {}: {}", entry.device.name, error.message);
    }
    println!(
        "{}\t{}\t{:?}\t{:?}\t{}\t{}\t{}",
        entry.device.id,
        entry.device.name,
        entry.device.match_status,
        entry.device.support_status,
        entry.device.provider_id.as_deref().unwrap_or("unknown"),
        battery,
        charge
    );
}

pub(super) fn print_json<T: serde::Serialize>(value: &T) -> Result<(), BatteryError> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value).map_err(|error| BatteryError::Internal {
        message: format!("JSON serialization failed: {error}"),
    })?;
    stdout
        .write_all(b"\n")
        .map_err(|error| BatteryError::Internal {
            message: format!("cannot write stdout: {error}"),
        })
}

pub(super) fn print_json_pretty<T: serde::Serialize>(value: &T) -> Result<(), BatteryError> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| BatteryError::Internal {
            message: format!("JSON serialization failed: {error}"),
        })?
    );
    Ok(())
}

pub(super) fn print_reading(reading: &BatteryReading) {
    let value = format_reading_value(reading);
    println!(
        "{}\t{}\t{:?}\t{:?}\t{:?}\t{}",
        reading.device_id,
        value,
        reading.charge_state,
        reading.reachability,
        reading.freshness,
        reading.observed_at.to_rfc3339()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Arc;
    use wireless_battery::model::{
        BatteryLevel, ChargeState, ClientConfig, DeviceId, Endpoint, EndpointId, Freshness,
        Precision, Reachability, ReadingSource,
    };
    use wireless_battery::registry::Registry;
    use wireless_battery::transport::replay::{
        Expectation, Fixture, FixtureEnvironment, FixtureSource, FixtureStep, FixtureTarget,
        Operation, ReplayError, ReplayTransport, StepOutcome,
    };
    use wireless_battery::transport::{HidSession, Transport, TransportError};

    struct MultiEndpointTransport {
        ok_ep: Endpoint,
        fail_ep: Endpoint,
        unknown_ep: Endpoint,
        ok_replay: ReplayTransport,
        fail_replay: ReplayTransport,
    }

    impl Transport for MultiEndpointTransport {
        fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
            Ok(vec![
                self.ok_ep.clone(),
                self.fail_ep.clone(),
                self.unknown_ep.clone(),
            ])
        }

        fn open(&self, endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
            if endpoint == &self.ok_ep.id {
                self.ok_replay.open(endpoint)
            } else if endpoint == &self.fail_ep.id {
                self.fail_replay.open(endpoint)
            } else {
                Err(TransportError::DeviceDisconnected)
            }
        }
    }

    #[test]
    fn format_reading_value_handles_percentage_level_voltage_and_none() {
        let base = BatteryReading {
            device_id: DeviceId("dev_test".to_owned()),
            endpoint_id: EndpointId("ep_test".to_owned()),
            percentage: None,
            level: None,
            voltage_mv: None,
            charge_state: ChargeState::Unknown,
            reachability: Reachability::Online,
            precision: Precision::None,
            source: ReadingSource::Mock,
            observed_at: Utc::now(),
            freshness: Freshness::Live,
        };
        assert_eq!(format_reading_value(&base), "unknown");
        assert_eq!(
            format_reading_value(&BatteryReading {
                percentage: Some(72),
                ..base.clone()
            }),
            "72%"
        );
        assert_eq!(
            format_reading_value(&BatteryReading {
                level: Some(BatteryLevel::Medium),
                ..base.clone()
            }),
            "medium"
        );
        assert_eq!(
            format_reading_value(&BatteryReading {
                voltage_mv: Some(3_850),
                ..base
            }),
            "3850mV"
        );
    }

    #[test]
    fn read_device_list_isolates_per_device_errors_without_live_hid() {
        let mut ok_ep = Endpoint::synthetic("ep_ok", 0x1234, 0x5678);
        ok_ep.usage_page = Some(0xFF00);
        ok_ep.product_name = Some("OK mouse".to_owned());

        let mut fail_ep = Endpoint::synthetic("ep_fail", 0x1234, 0x5678);
        fail_ep.usage_page = Some(0xFF00);
        fail_ep.product_name = Some("Timeout mouse".to_owned());

        let unknown_ep = Endpoint::synthetic("ep_unknown", 0x9999, 0x0001);

        let ok_fixture = Fixture {
            schema_version: 1,
            target: FixtureTarget {
                kind: "provider".to_owned(),
                id: "synthetic.feature.mouse".to_owned(),
            },
            case: "ok".to_owned(),
            source: FixtureSource::Synthetic,
            environment: FixtureEnvironment {
                os: None,
                connection: "synthetic".to_owned(),
                firmware: None,
            },
            endpoint: ok_ep.clone(),
            steps: vec![
                FixtureStep {
                    operation: Operation::SendFeature,
                    report_id: Some(9),
                    request_hex: Some("01 00 00 00".to_owned()),
                    length: None,
                    outcome: StepOutcome::Ok,
                },
                FixtureStep {
                    operation: Operation::GetFeature,
                    report_id: Some(9),
                    request_hex: None,
                    length: Some(4),
                    outcome: StepOutcome::Response {
                        report_id: 9,
                        hex: "01 48 01 4A".to_owned(),
                    },
                },
            ],
            expect: Expectation::Reading {
                percentage: Some(72),
                level: None,
                voltage_mv: None,
                charge_state: ChargeState::Charging,
                reachability: Reachability::Online,
                precision: Precision::Percentage,
            },
        };
        let fail_fixture = Fixture {
            endpoint: fail_ep.clone(),
            steps: vec![
                FixtureStep {
                    operation: Operation::SendFeature,
                    report_id: Some(9),
                    request_hex: Some("01 00 00 00".to_owned()),
                    length: None,
                    outcome: StepOutcome::Ok,
                },
                FixtureStep {
                    operation: Operation::GetFeature,
                    report_id: Some(9),
                    request_hex: None,
                    length: Some(4),
                    outcome: StepOutcome::Error {
                        error: ReplayError::Timeout,
                    },
                },
            ],
            ..ok_fixture.clone()
        };
        let transport = Arc::new(MultiEndpointTransport {
            ok_ep,
            fail_ep,
            unknown_ep,
            ok_replay: ReplayTransport::new(&ok_fixture),
            fail_replay: ReplayTransport::new(&fail_fixture),
        });
        let client = BatteryClient::with_transport(
            ClientConfig {
                allow_experimental: true,
                enable_cache: false,
                ..ClientConfig::default()
            },
            transport,
            Registry::with_synthetic().unwrap(),
        )
        .unwrap();
        let devices = client.list_devices().unwrap();
        let entries = read_device_list(&client, devices, 1_000);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].reading.as_ref().unwrap().percentage, Some(72));
        assert!(entries[0].read_error.is_none());
        assert!(entries[1].reading.is_none());
        assert_eq!(
            entries[1].read_error.as_ref().unwrap().kind,
            wireless_battery::ErrorKind::Timeout
        );
        assert!(entries[2].reading.is_none());
        assert!(entries[2].read_error.is_none());
    }
}
