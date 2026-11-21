mod common;

use common::{fixture, root, run_fixture};
use wireless_battery::package::{load_package, parse_package, validate_package};
use wireless_battery::transport::replay::{Expectation, StepOutcome};
use wireless_battery::ErrorKind;

fn package_text() -> String {
    std::fs::read_to_string(root().join("fixtures/synthetic/packages/feature_mouse.yaml")).unwrap()
}

#[test]
fn package_and_evidence_validate_from_repository_root() {
    let package = load_package(
        &root().join("fixtures/synthetic/packages/feature_mouse.yaml"),
        &root(),
    )
    .unwrap();
    assert_eq!(package.id, "synthetic.feature.mouse");
}

#[test]
fn rejects_field_overflow_unknown_members_and_verified_without_evidence() {
    for (from, to) in [
        ("offset: 1\n    type: u8", "offset: 99\n    type: u8"),
        (
            "name: Synthetic Feature Mouse",
            "name: Synthetic Feature Mouse\nunexpected: true",
        ),
        ("source: synthetic", "source: real"),
    ] {
        let mut yaml = package_text().replacen(from, to, 1);
        if from == "source: synthetic" {
            yaml = yaml.replacen("status: experimental", "status: verified", 1);
            yaml = yaml.replacen(
                "  provenance: provenance/synthetic.feature.mouse.md\n",
                "",
                1,
            );
        }
        assert!(
            parse_package(&yaml, "invalid.yaml").is_err(),
            "should reject: {to}"
        );
    }
}

#[test]
fn rejects_duplicate_steps_excess_steps_and_path_escape() {
    let package = parse_package(&package_text(), "steps.yaml").unwrap();
    let mut duplicate = package.clone();
    duplicate.query.steps = vec![
        package.query.steps[0].clone(),
        package.query.steps[0].clone(),
    ];
    assert!(validate_package(&duplicate, "duplicate.yaml", None).is_err());
    let mut excess = package.clone();
    excess.query.steps = vec![package.query.steps[0].clone(); 5];
    assert!(validate_package(&excess, "five-steps.yaml", None).is_err());

    let mut package = package;
    package.evidence.fixtures.push("../outside.json".to_owned());
    assert!(validate_package(&package, "paths.yaml", Some(&root())).is_err());
}

#[test]
fn percentage_zero_and_one_hundred_are_valid() {
    for (percentage, hex) in [(0, "01 00 00 01"), (100, "01 64 00 65")] {
        let mut fixture = fixture("feature_normal.json");
        fixture.steps[1].outcome = StepOutcome::Response {
            report_id: 9,
            hex: hex.to_owned(),
        };
        fixture.expect = Expectation::Reading {
            percentage: Some(percentage),
            level: None,
            voltage_mv: None,
            charge_state: wireless_battery::ChargeState::Discharging,
            reachability: wireless_battery::Reachability::Online,
            precision: wireless_battery::Precision::Percentage,
        };
        let (result, replay) = run_fixture(&fixture);
        common::assert_expected(&fixture, &result);
        replay.finish().unwrap();
    }
}

#[test]
fn empty_long_and_wrong_report_id_responses_fail_safely() {
    for (report_id, hex) in [(9, ""), (9, "01 48 01 4A 00"), (8, "01 48 01 4A")] {
        let mut fixture = fixture("feature_normal.json");
        fixture.steps[1].outcome = StepOutcome::Response {
            report_id,
            hex: hex.to_owned(),
        };
        let (result, _) = run_fixture(&fixture);
        assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidResponse);
    }
}

#[test]
fn xor_checksum_and_permission_errors_are_supported() {
    let package = package_text().replacen("algorithm: sum8", "algorithm: xor8", 1);
    assert!(parse_package(&package, "xor.yaml").is_ok());

    let mut fixture = fixture("feature_normal.json");
    fixture.steps[1].outcome = StepOutcome::Error {
        error: wireless_battery::transport::replay::ReplayError::PermissionDenied,
    };
    let (result, replay) = run_fixture(&fixture);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::PermissionDenied);
    replay.finish().unwrap();
}

#[test]
fn zero_total_timeout_is_distinct_from_device_data() {
    let mut fixture = fixture("feature_normal.json");
    fixture.expect = Expectation::Error {
        error: ErrorKind::Timeout,
    };
    let replay = std::sync::Arc::new(wireless_battery::transport::replay::ReplayTransport::new(
        &fixture,
    ));
    let client = wireless_battery::BatteryClient::with_transport(
        wireless_battery::ClientConfig {
            allow_experimental: true,
            ..wireless_battery::ClientConfig::default()
        },
        replay,
        wireless_battery::registry::Registry::with_synthetic().unwrap(),
    )
    .unwrap();
    let device = client.list_devices().unwrap().remove(0);
    let error = client
        .read_battery_with_options(
            &device.id.0,
            wireless_battery::ReadOptions {
                timeout: std::time::Duration::ZERO,
                allow_stale: false,
                allow_experimental: true,
            },
        )
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Timeout);
}

#[test]
fn output_input_level_voltage_xor_and_reserved_enum_work() {
    run_output_case("AA 03 90 04 3D", wireless_battery::ChargeState::Discharging);
    run_output_case("AA 03 90 09 30", wireless_battery::ChargeState::Unknown);
}

fn run_output_case(response_hex: &str, expected_charge: wireless_battery::ChargeState) {
    let yaml = r#"
schema_version: 1
id: synthetic.output.level
version: 1.0.0
status: experimental
name: Synthetic Output Level Device
source: synthetic
match:
  all: { vid: 0x1234, pid: 0x5678, usage_page: 0xFF00 }
capabilities:
  percentage: false
  discrete_level: true
  voltage: true
  charge_state: true
  reachability: false
transport: { kind: hid_output, timeout_ms: 200, cooldown_ms: 0 }
query:
  steps:
    - id: battery
      write_output: { report_id: 2, payload_hex: "10" }
      read_input: { report_id: 4, length: 5 }
response:
  report_id: 4
  length: 5
  constants: [{ offset: 0, hex: "AA" }]
  checksum: { algorithm: xor8, range: [0, 3], offset: 4 }
fields:
  level:
    offset: 1
    type: u8
    unknown_values: [255]
    map: { "0": critical, "1": low, "2": medium, "3": high, "4": full }
  voltage_mv:
    offset: 2
    type: u8
    scale: 25
  charge_state:
    offset: 3
    type: u8
    map: { "0": unknown, "1": charging, "2": not_charging, "3": full, "4": discharging }
"#;
    let provider = wireless_battery::providers::declarative::DeclarativeProvider::from_yaml(
        yaml,
        "output.yaml",
    )
    .unwrap();
    let registry =
        wireless_battery::registry::Registry::new(vec![std::sync::Arc::new(provider)]).unwrap();
    let mut fixture = fixture("feature_normal.json");
    fixture.target.id = "synthetic.output.level".to_owned();
    fixture.steps = vec![
        wireless_battery::transport::replay::FixtureStep {
            operation: wireless_battery::transport::replay::Operation::WriteOutput,
            report_id: Some(2),
            request_hex: Some("10".to_owned()),
            length: None,
            outcome: StepOutcome::Ok,
        },
        wireless_battery::transport::replay::FixtureStep {
            operation: wireless_battery::transport::replay::Operation::ReadInput,
            report_id: None,
            request_hex: None,
            length: None,
            outcome: StepOutcome::Response {
                report_id: 4,
                hex: response_hex.to_owned(),
            },
        },
    ];
    let replay = std::sync::Arc::new(wireless_battery::transport::replay::ReplayTransport::new(
        &fixture,
    ));
    let client = wireless_battery::BatteryClient::with_transport(
        wireless_battery::ClientConfig {
            allow_experimental: true,
            ..wireless_battery::ClientConfig::default()
        },
        replay.clone(),
        registry,
    )
    .unwrap();
    let device = client.list_devices().unwrap().remove(0);
    let reading = client.read_battery(&device.id.0).unwrap();
    assert_eq!(reading.percentage, None);
    assert_eq!(reading.level, Some(wireless_battery::BatteryLevel::High));
    assert_eq!(reading.voltage_mv, Some(3_600));
    assert_eq!(reading.charge_state, expected_charge);
    assert_eq!(
        reading.precision,
        wireless_battery::Precision::DiscreteLevel
    );
    replay.finish().unwrap();
}

#[test]
fn rejects_u8_bool_bit_overlap_and_triple_bit_collision() {
    // u8 percentage at offset 1 overlapping with bool_bit charge_state at offset 1 bit 7.
    let u8_then_bit = package_text().replacen(
        "  charge_state:\n    offset: 2\n    type: bool_bit\n    bit: 0",
        "  charge_state:\n    offset: 1\n    type: bool_bit\n    bit: 7",
        1,
    );
    assert!(parse_package(&u8_then_bit, "u8-bit-overlap.yaml").is_err());

    // Three bool_bit fields on the same offset where the 1st and 3rd collide on bit 0.
    let triple_bits = r#"
schema_version: 1
id: synthetic.triple.bits
version: 1.0.0
status: experimental
name: Synthetic Triple Bits
source: synthetic
match:
  all: { vid: 0x1234, pid: 0x5679, usage_page: 0xFF00 }
capabilities:
  percentage: false
  discrete_level: true
  voltage: false
  charge_state: true
  reachability: true
transport: { kind: hid_feature, timeout_ms: 200, cooldown_ms: 0 }
query:
  steps:
    - id: battery
      send_feature: { report_id: 1, payload_hex: "01" }
      receive_feature: { report_id: 1, length: 2 }
response:
  report_id: 1
  length: 2
fields:
  level:
    offset: 0
    type: bool_bit
    bit: 0
    map: { "true": full, "false": low }
  charge_state:
    offset: 0
    type: bool_bit
    bit: 1
    map: { "true": charging, "false": discharging }
  reachability:
    offset: 0
    type: bool_bit
    bit: 0
    map: { "true": online, "false": offline }
"#;
    assert!(parse_package(triple_bits, "triple-bits.yaml").is_err());
}

#[test]
fn hid_output_checks_report_id_before_checksum() {
    let yaml = r#"
schema_version: 1
id: synthetic.output.report_id_order
version: 1.0.0
status: experimental
name: Synthetic Output Report ID Order
source: synthetic
match:
  all: { vid: 0x1234, pid: 0x5678, usage_page: 0xFF00 }
capabilities:
  percentage: true
  discrete_level: false
  voltage: false
  charge_state: false
  reachability: false
transport: { kind: hid_output, timeout_ms: 200, cooldown_ms: 0 }
query:
  steps:
    - id: battery
      write_output: { report_id: 2, payload_hex: "10" }
      read_input: { report_id: 4, length: 2, checksum: { algorithm: xor8, range: [0, 0], offset: 1 } }
response:
  report_id: 4
  length: 2
  checksum: { algorithm: xor8, range: [0, 0], offset: 1 }
fields:
  percentage:
    offset: 0
    type: u8
"#;
    let provider = wireless_battery::providers::declarative::DeclarativeProvider::from_yaml(
        yaml,
        "report-id-order.yaml",
    )
    .unwrap();
    let registry =
        wireless_battery::registry::Registry::new(vec![std::sync::Arc::new(provider)]).unwrap();
    let mut fixture = fixture("feature_normal.json");
    fixture.steps = vec![
        wireless_battery::transport::replay::FixtureStep {
            operation: wireless_battery::transport::replay::Operation::WriteOutput,
            report_id: Some(2),
            request_hex: Some("10".to_owned()),
            length: None,
            outcome: StepOutcome::Ok,
        },
        wireless_battery::transport::replay::FixtureStep {
            operation: wireless_battery::transport::replay::Operation::ReadInput,
            report_id: None,
            request_hex: None,
            length: None,
            outcome: StepOutcome::Response {
                // Wrong report ID (9 != 4) AND bad checksum (0x32 != 0xFF):
                // must fail with InvalidResponse, not ChecksumFailed.
                report_id: 9,
                hex: "32 FF".to_owned(),
            },
        },
    ];
    let replay = std::sync::Arc::new(wireless_battery::transport::replay::ReplayTransport::new(
        &fixture,
    ));
    let client = wireless_battery::BatteryClient::with_transport(
        wireless_battery::ClientConfig {
            allow_experimental: true,
            ..wireless_battery::ClientConfig::default()
        },
        replay,
        registry,
    )
    .unwrap();
    let device = client.list_devices().unwrap().remove(0);
    let err = client.read_battery(&device.id.0).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidResponse);
}
