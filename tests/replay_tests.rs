mod common;

use common::{assert_expected, fixture, run_fixture};
use std::sync::Arc;
use std::time::Duration;
use wireless_battery::model::{
    Capabilities, ChargeState, ClientConfig, Endpoint, ProviderReading, ReadingSource,
    SupportStatus,
};
use wireless_battery::providers::{
    Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata,
};
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::{
    Expectation, Fixture, FixtureEnvironment, FixtureSource, FixtureStep, FixtureTarget, Operation,
    ReplayError, ReplayTransport, StepOutcome,
};
use wireless_battery::transport::{HidSession, Transport};
use wireless_battery::{BatteryClient, BatteryError, ErrorKind};

#[test]
fn all_embedded_synthetic_cases_replay() {
    for name in [
        "feature_normal.json",
        "feature_truncated.json",
        "feature_unknown.json",
        "feature_out_of_range.json",
        "feature_checksum.json",
        "feature_timeout.json",
        "feature_disconnected.json",
        "bluetooth_bas.json",
    ] {
        let fixture = fixture(name);
        let (result, replay) = run_fixture(&fixture);
        assert_expected(&fixture, &result);
        replay
            .finish()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn replay_rejects_changed_request_bytes() {
    let fixture = fixture("feature_normal.json");
    let replay = ReplayTransport::new(&fixture);
    let endpoint = replay.enumerate().unwrap().remove(0);
    let mut session = replay.open(&endpoint.id).unwrap();
    let error = session.send_feature(9, &[1, 0]).unwrap_err();
    assert!(error.to_string().contains("step 1"));
}

#[test]
fn replay_rejects_extra_and_omitted_requests() {
    let fixture = fixture("feature_normal.json");
    let replay = ReplayTransport::new(&fixture);
    let endpoint = replay.enumerate().unwrap().remove(0);
    let mut session = replay.open(&endpoint.id).unwrap();
    session.send_feature(9, &[1, 0, 0, 0]).unwrap();
    let error = session.send_feature(9, &[1, 0, 0, 0]).unwrap_err();
    assert!(error.to_string().contains("step 2"));

    let replay = ReplayTransport::new(&fixture);
    assert!(replay.finish().unwrap_err().contains("omitted 2"));
}

#[test]
fn multi_step_provider_uses_dynamic_index_and_fallback() {
    for (negotiated, expected_id) in [(9, 9), (255, 2)] {
        let fixture = multi_step_fixture(negotiated, expected_id, false);
        let provider = Arc::new(SyntheticMultiStepProvider);
        let registry = Registry::new(vec![provider]).unwrap();
        let replay = Arc::new(ReplayTransport::new(&fixture));
        let client =
            BatteryClient::with_transport(ClientConfig::default(), replay.clone(), registry)
                .unwrap();
        let device = client.list_devices().unwrap().remove(0);
        let reading = client.read_battery(&device.id.0).unwrap();
        assert_eq!(reading.percentage, Some(64));
        assert_eq!(reading.charge_state, ChargeState::Discharging);
        replay.finish().unwrap();
    }
}

#[test]
fn multi_step_second_round_timeout_is_not_zero_percent() {
    let fixture = multi_step_fixture(9, 9, true);
    let registry = Registry::new(vec![Arc::new(SyntheticMultiStepProvider)]).unwrap();
    let replay = Arc::new(ReplayTransport::new(&fixture));
    let client =
        BatteryClient::with_transport(ClientConfig::default(), replay.clone(), registry).unwrap();
    let device = client.list_devices().unwrap().remove(0);
    let error = client.read_battery(&device.id.0).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Timeout);
    replay.finish().unwrap();
}

struct SyntheticMultiStepProvider;

impl Provider for SyntheticMultiStepProvider {
    fn id(&self) -> &str {
        "synthetic.multi_step"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: "1.0.0".to_owned(),
            device_kind: wireless_battery::DeviceKind::Mouse,
            support_status: SupportStatus::Verified,
            capabilities: Capabilities {
                percentage: true,
                charge_state: true,
                ..Capabilities::default()
            },
            io_timeout: Duration::from_millis(100),
            cooldown: Duration::ZERO,
        }
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x6666) && endpoint.pid == Some(0x0001) {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        deadline.check("negotiate")?;
        session
            .send_feature(1, &[1])
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        let negotiation = session
            .get_feature(1, 1)
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        let index = if negotiation == [255] {
            2
        } else {
            negotiation[0]
        };
        deadline.check("battery query")?;
        session
            .send_feature(index, &[2])
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        let response = session
            .get_feature(index, 2)
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        if response[0] > 100 {
            return Err(BatteryError::InvalidResponse {
                reason: "percentage above 100".to_owned(),
            });
        }
        Ok(ProviderReading {
            percentage: Some(response[0]),
            charge_state: if response[1] == 0 {
                ChargeState::Discharging
            } else {
                ChargeState::Charging
            },
            source: ReadingSource::VendorHid,
            ..ProviderReading::default()
        })
    }
}

fn multi_step_fixture(negotiated: u8, query_id: u8, timeout: bool) -> Fixture {
    let outcome = if timeout {
        StepOutcome::Error {
            error: ReplayError::Timeout,
        }
    } else {
        StepOutcome::Response {
            report_id: query_id,
            hex: "40 00".to_owned(),
        }
    };
    Fixture {
        schema_version: 1,
        target: FixtureTarget {
            kind: "provider".to_owned(),
            id: "synthetic.multi_step".to_owned(),
        },
        case: "dynamic_multi_step".to_owned(),
        source: FixtureSource::Synthetic,
        environment: FixtureEnvironment {
            os: None,
            connection: "synthetic".to_owned(),
            firmware: None,
        },
        endpoint: Endpoint::synthetic("ep_multi", 0x6666, 1),
        steps: vec![
            FixtureStep {
                operation: Operation::SendFeature,
                report_id: Some(1),
                request_hex: Some("01".to_owned()),
                length: None,
                outcome: StepOutcome::Ok,
            },
            FixtureStep {
                operation: Operation::GetFeature,
                report_id: Some(1),
                request_hex: None,
                length: Some(1),
                outcome: StepOutcome::Response {
                    report_id: 1,
                    hex: format!("{negotiated:02X}"),
                },
            },
            FixtureStep {
                operation: Operation::SendFeature,
                report_id: Some(query_id),
                request_hex: Some("02".to_owned()),
                length: None,
                outcome: StepOutcome::Ok,
            },
            FixtureStep {
                operation: Operation::GetFeature,
                report_id: Some(query_id),
                request_hex: None,
                length: Some(2),
                outcome,
            },
        ],
        expect: Expectation::Error {
            error: ErrorKind::Timeout,
        },
    }
}
