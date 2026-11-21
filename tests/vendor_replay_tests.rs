mod common;

use common::{assert_expected, load_fixture, root, run_fixture};
use std::path::{Path, PathBuf};
use wireless_battery::transport::replay::{FixtureStep, Operation, StepOutcome};
use wireless_battery::{ChargeState, ErrorKind, HidBusType, Precision};

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_json_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            out.push(path);
        }
    }
}

#[test]
fn every_vendor_provider_has_a_strict_json_replay() {
    let mut paths = Vec::new();
    collect_json_files(&root().join("fixtures/vendors"), &mut paths);
    paths.sort();
    assert!(paths.len() >= 24, "all vendor providers need fixtures");
    for path in paths {
        let fixture = load_fixture(&path);
        let (result, replay) = run_fixture(&fixture);
        assert_expected(&fixture, &result);
        replay
            .finish()
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }
}

#[test]
fn logitech_1004_reads_charge_status_from_byte_6_and_fast_fails_hidpp_errors() {
    // Low battery level bitmask (response[5] = 0x02) while discharging (response[6] = 0x00)
    // must report Discharging, NOT Full.
    let mut low_discharging =
        load_fixture(&root().join("fixtures/vendors/logitech/hidpp_1004.json"));
    low_discharging.steps.last_mut().unwrap().outcome = StepOutcome::Response {
        report_id: 17,
        hex: "01 05 1E 0F 02 00 00 00 00 00 00 00 00 00 00 00 00 00 00".to_owned(),
    };
    let (result, replay) = run_fixture(&low_discharging);
    let reading = result.unwrap();
    assert_eq!(reading.percentage, Some(15));
    assert_eq!(reading.charge_state, ChargeState::Discharging);
    replay.finish().unwrap();

    // Charging (response[5] = 0x04 good, response[6] = 0x01 charging) must report Charging.
    let mut charging = load_fixture(&root().join("fixtures/vendors/logitech/hidpp_1004.json"));
    charging.steps.last_mut().unwrap().outcome = StepOutcome::Response {
        report_id: 17,
        hex: "01 05 1E 32 04 01 00 00 00 00 00 00 00 00 00 00 00 00 00".to_owned(),
    };
    let (result, replay) = run_fixture(&charging);
    let reading = result.unwrap();
    assert_eq!(reading.percentage, Some(50));
    assert_eq!(reading.charge_state, ChargeState::Charging);
    replay.finish().unwrap();

    // HID++ 2.0 error frame (report[2] == 0xFF, report[3] == feature, report[4] == func|swid)
    // must fast-fail with InvalidResponse instead of hanging until timeout.
    let mut hidpp_error = load_fixture(&root().join("fixtures/vendors/logitech/hidpp_1004.json"));
    hidpp_error.steps.last_mut().unwrap().outcome = StepOutcome::Response {
        report_id: 17,
        hex: "01 FF 05 1E 09 00 00 00 00 00 00 00 00 00 00 00 00 00 00".to_owned(),
    };
    let (result, replay) = run_fixture(&hidpp_error);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidResponse);
    replay.finish().unwrap();
}

#[test]
fn razer_scales_0_to_255_monotonically_and_reports_checksum_failed() {
    let base = load_fixture(&root().join("fixtures/vendors/razer/mouse.json"));
    let mut last_pct = 0_u8;
    for (raw, expected) in [
        (0_u8, 0_u8),
        (76, 30),
        (100, 39),
        (101, 40),
        (191, 75),
        (255, 100),
    ] {
        let mut fixture = base.clone();
        let mut payload = vec![0_u8; 90];
        payload[0] = 0x02;
        payload[1] = 0x20;
        payload[5] = 0x02;
        payload[6] = 0x07;
        payload[7] = 0x80;
        payload[8] = 0x00;
        payload[9] = raw;
        payload[88] = payload[2..=87].iter().fold(0_u8, |crc, byte| crc ^ byte);
        fixture.steps[1].outcome = StepOutcome::Response {
            report_id: 0,
            hex: wireless_battery::transport::replay::Fixture::encode_hex(&payload),
        };
        let (result, replay) = run_fixture(&fixture);
        let pct = result.unwrap().percentage.unwrap();
        assert_eq!(pct, expected);
        assert!(pct >= last_pct);
        last_pct = pct;
        replay.finish().unwrap();
    }

    // Corrupt CRC byte must yield ChecksumFailed.
    let mut bad_crc = base;
    bad_crc.steps[1].outcome = StepOutcome::Response {
        report_id: 0,
        hex: "02 20 00 00 00 02 07 80 00 BF 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 FF 00".to_owned(),
    };
    let (result, replay) = run_fixture(&bad_crc);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::ChecksumFailed);
    replay.finish().unwrap();
}

#[test]
fn dualsense_bluetooth_uses_get_feature_and_unknown_status_stays_unknown() {
    let mut fixture = load_fixture(&root().join("fixtures/vendors/sony/dualsense.json"));
    fixture.endpoint.bus_type = Some(HidBusType::Bluetooth);
    let mut report = vec![0_u8; 63];
    // For report_id 0x31, frame[54] == payload[53]. High nibble 0x05 is unknown/reserved.
    report[53] = 0x57;
    fixture.steps = vec![
        FixtureStep {
            operation: Operation::GetFeature,
            report_id: Some(0x05),
            request_hex: None,
            length: Some(40),
            outcome: StepOutcome::Response {
                report_id: 0x05,
                hex: wireless_battery::transport::replay::Fixture::encode_hex(&[0_u8; 40]),
            },
        },
        FixtureStep {
            operation: Operation::ReadInput,
            report_id: None,
            request_hex: None,
            length: None,
            outcome: StepOutcome::Response {
                report_id: 0x31,
                hex: wireless_battery::transport::replay::Fixture::encode_hex(&report),
            },
        },
    ];
    let (result, replay) = run_fixture(&fixture);
    let reading = result.unwrap();
    assert_eq!(reading.percentage, Some(70));
    assert_eq!(reading.charge_state, ChargeState::Unknown);
    replay.finish().unwrap();
}

#[test]
fn interface_number_matching_survives_capture_serialization_without_synthetic_path() {
    for path in [
        "fixtures/vendors/mchose/k7.json",
        "fixtures/vendors/asus/rog_omni.json",
    ] {
        let mut fixture = load_fixture(&root().join(path));
        fixture.endpoint.interface_number = Some(2);
        // Round-trip through JSON serialization, which strips `system_path`.
        let serialized = serde_json::to_string(&fixture).unwrap();
        let reloaded: wireless_battery::transport::replay::Fixture =
            serde_json::from_str(&serialized).unwrap();
        let (result, replay) = run_fixture(&reloaded);
        assert_expected(&reloaded, &result);
        replay.finish().unwrap();
    }
}

#[test]
fn xinput_and_bas_replay_validate_errors_and_edge_cases() {
    // Disconnected XInput (battery_type = 0) -> DeviceDisconnected
    let mut disconnected = load_fixture(&root().join("fixtures/vendors/os/xinput_slot0.json"));
    disconnected.steps[0].outcome = StepOutcome::XInput {
        battery_type: 0,
        battery_level: 0,
    };
    let (result, replay) = run_fixture(&disconnected);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::DeviceDisconnected);
    replay.finish().unwrap();

    // Wired XInput (battery_type = 1) -> Precision::None, level = None
    let mut wired = load_fixture(&root().join("fixtures/vendors/os/xinput_slot0.json"));
    wired.steps[0].outcome = StepOutcome::XInput {
        battery_type: 1,
        battery_level: 0,
    };
    let (result, replay) = run_fixture(&wired);
    let reading = result.unwrap();
    assert_eq!(reading.level, None);
    assert_eq!(reading.precision, Precision::None);
    replay.finish().unwrap();

    // Invalid XInput level (9) -> InvalidResponse
    let mut bad_level = load_fixture(&root().join("fixtures/vendors/os/xinput_slot0.json"));
    bad_level.steps[0].outcome = StepOutcome::XInput {
        battery_type: 2,
        battery_level: 9,
    };
    let (result, replay) = run_fixture(&bad_level);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidResponse);
    replay.finish().unwrap();

    // Out-of-range BAS (> 100%) -> InvalidResponse
    let mut bad_bas = load_fixture(&root().join("fixtures/synthetic/bluetooth_bas.json"));
    bad_bas.steps[0].outcome = StepOutcome::Bas { percentage: 120 };
    let (result, replay) = run_fixture(&bad_bas);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidResponse);
    replay.finish().unwrap();
}

#[test]
fn wob_rainy75_reads_charge_state_from_byte_9_and_keeps_unknown_values_unknown() {
    // Discharging real capture (payload byte 8 / frame byte 9 == 0x00)
    let discharging = load_fixture(&root().join("fixtures/vendors/wob/rainy75_real.json"));
    let (result, replay) = run_fixture(&discharging);
    let reading = result.unwrap();
    assert_eq!(reading.percentage, Some(50));
    assert_eq!(reading.charge_state, ChargeState::Discharging);
    replay.finish().unwrap();

    // Charging real capture (payload byte 8 / frame byte 9 == 0x01)
    let charging = load_fixture(&root().join("fixtures/vendors/wob/rainy75_charging_real.json"));
    let (result, replay) = run_fixture(&charging);
    let reading = result.unwrap();
    assert_eq!(reading.percentage, Some(62));
    assert_eq!(reading.charge_state, ChargeState::Charging);
    replay.finish().unwrap();

    // Unverified/reserved charge flag (0x02) must stay Unknown rather than guessing Full.
    let mut reserved = charging;
    reserved.steps.last_mut().unwrap().outcome = StepOutcome::Response {
        report_id: 4,
        hex: "20 00 1A 06 00 00 00 64 02 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00".to_owned(),
    };
    let (result, replay) = run_fixture(&reserved);
    let reading = result.unwrap();
    assert_eq!(reading.percentage, Some(100));
    assert_eq!(reading.charge_state, ChargeState::Unknown);
    replay.finish().unwrap();
}
