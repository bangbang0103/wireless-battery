//! Shared synthetic fixture builders and helpers for extended vendor replay tests.

use wireless_battery::providers::{MatchDecision, Provider};
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::{
    verify_fixture, Expectation, Fixture, FixtureEnvironment, FixtureSource, FixtureStep,
    FixtureTarget, Operation, ReplayError, StepOutcome,
};
use wireless_battery::{ChargeState, Endpoint, Precision, Reachability};

pub(super) fn ep(
    id: &str,
    vid: u16,
    pid: u16,
    usage_page: Option<u16>,
    usage: Option<u16>,
    interface_number: Option<i32>,
    product_name: Option<&str>,
) -> Endpoint {
    let mut e = Endpoint::synthetic(format!("ep_{id}"), vid, pid);
    e.usage_page = usage_page;
    e.usage = usage;
    e.interface_number = interface_number;
    e.product_name = product_name.map(ToOwned::to_owned);
    e.manufacturer = Some("Synthetic".to_owned());
    e
}

pub(super) fn step_flush_timeout() -> FixtureStep {
    FixtureStep {
        operation: Operation::ReadInput,
        report_id: None,
        request_hex: None,
        length: None,
        outcome: StepOutcome::Error {
            error: ReplayError::Timeout,
        },
    }
}

pub(super) fn step_write(report_id: u8, payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::WriteOutput,
        report_id: Some(report_id),
        request_hex: Some(Fixture::encode_hex(payload)),
        length: None,
        outcome: StepOutcome::Ok,
    }
}

pub(super) fn step_set_output(report_id: u8, payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::SetOutput,
        report_id: Some(report_id),
        request_hex: Some(Fixture::encode_hex(payload)),
        length: None,
        outcome: StepOutcome::Ok,
    }
}

pub(super) fn step_send_feature(report_id: u8, payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::SendFeature,
        report_id: Some(report_id),
        request_hex: Some(Fixture::encode_hex(payload)),
        length: None,
        outcome: StepOutcome::Ok,
    }
}

pub(super) fn step_get_feature(report_id: u8, length: usize, resp_payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::GetFeature,
        report_id: Some(report_id),
        request_hex: None,
        length: Some(length),
        outcome: StepOutcome::Response {
            report_id,
            hex: Fixture::encode_hex(resp_payload),
        },
    }
}

pub(super) fn step_get_input(report_id: u8, length: usize, resp_payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::GetInput,
        report_id: Some(report_id),
        request_hex: None,
        length: Some(length),
        outcome: StepOutcome::Response {
            report_id,
            hex: Fixture::encode_hex(resp_payload),
        },
    }
}

pub(super) fn step_read(report_id: u8, payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::ReadInput,
        report_id: None,
        request_hex: None,
        length: None,
        outcome: StepOutcome::Response {
            report_id,
            hex: Fixture::encode_hex(payload),
        },
    }
}

pub(super) fn make_fixture(
    provider_id: &str,
    endpoint: Endpoint,
    steps: Vec<FixtureStep>,
    expect: Expectation,
) -> Fixture {
    Fixture {
        schema_version: 1,
        target: FixtureTarget {
            kind: "provider".to_owned(),
            id: provider_id.to_owned(),
        },
        case: format!("{provider_id}_synthetic"),
        source: FixtureSource::Synthetic,
        environment: FixtureEnvironment {
            os: None,
            connection: "synthetic".to_owned(),
            firmware: None,
        },
        endpoint,
        steps,
        expect,
    }
}

pub(super) fn expect_pct(
    pct: u8,
    charge_state: ChargeState,
    voltage_mv: Option<u32>,
) -> Expectation {
    Expectation::Reading {
        percentage: Some(pct),
        level: None,
        voltage_mv,
        charge_state,
        reachability: Reachability::Online,
        precision: Precision::Percentage,
    }
}

pub(super) fn crc16(bytes: &[u8], init: u16) -> u16 {
    let mut crc = init;
    for &b in bytes {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            crc = if (crc & 0x8000) != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub(super) fn sum8(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0_u8, |a, b| a.wrapping_add(*b))
}

pub(super) fn padded(prefix: &[u8], total_len: usize) -> Vec<u8> {
    let mut v = vec![0_u8; total_len];
    v[..prefix.len()].copy_from_slice(prefix);
    v
}

pub(super) fn match_providers<'a>(
    registry: &'a Registry,
    endpoint: &Endpoint,
) -> Vec<(&'a dyn Provider, MatchDecision)> {
    registry
        .providers()
        .iter()
        .filter_map(|provider| {
            let decision = provider.matches(endpoint);
            (decision != MatchDecision::NoMatch).then_some((provider.as_ref(), decision))
        })
        .collect()
}

pub(super) fn verify_all(registry: &Registry, fixtures: &[Fixture]) {
    for fixture in fixtures {
        verify_fixture(fixture, registry.clone())
            .unwrap_or_else(|err| panic!("fixture {} failed: {err}", fixture.case));
    }
}
