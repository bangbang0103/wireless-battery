//! Strict replay and zero-conflict match verification for all extended built-in vendor providers.

use std::collections::HashSet;
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::{
    verify_fixture, Expectation, Fixture, FixtureEnvironment, FixtureSource, FixtureStep,
    FixtureTarget, Operation, ReplayError, StepOutcome,
};
use wireless_battery::{BatteryLevel, ChargeState, Endpoint, Precision, Reachability};

fn ep(
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

fn step_flush_timeout() -> FixtureStep {
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

fn step_write(report_id: u8, payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::WriteOutput,
        report_id: Some(report_id),
        request_hex: Some(Fixture::encode_hex(payload)),
        length: None,
        outcome: StepOutcome::Ok,
    }
}

fn step_set_output(report_id: u8, payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::SetOutput,
        report_id: Some(report_id),
        request_hex: Some(Fixture::encode_hex(payload)),
        length: None,
        outcome: StepOutcome::Ok,
    }
}

fn step_send_feature(report_id: u8, payload: &[u8]) -> FixtureStep {
    FixtureStep {
        operation: Operation::SendFeature,
        report_id: Some(report_id),
        request_hex: Some(Fixture::encode_hex(payload)),
        length: None,
        outcome: StepOutcome::Ok,
    }
}

fn step_get_feature(report_id: u8, length: usize, resp_payload: &[u8]) -> FixtureStep {
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

fn step_get_input(report_id: u8, length: usize, resp_payload: &[u8]) -> FixtureStep {
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

fn step_read(report_id: u8, payload: &[u8]) -> FixtureStep {
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

fn make_fixture(
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

fn expect_pct(pct: u8, charge_state: ChargeState, voltage_mv: Option<u32>) -> Expectation {
    Expectation::Reading {
        percentage: Some(pct),
        level: None,
        voltage_mv,
        charge_state,
        reachability: Reachability::Online,
        precision: Precision::Percentage,
    }
}

fn crc16(bytes: &[u8], init: u16) -> u16 {
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

fn sum8(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0_u8, |a, b| a.wrapping_add(*b))
}

fn padded(prefix: &[u8], total_len: usize) -> Vec<u8> {
    let mut v = vec![0_u8; total_len];
    v[..prefix.len()].copy_from_slice(prefix);
    v
}

#[test]
fn extended_vendor_providers_replay_and_match_without_conflict() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    let mut fixtures: Vec<Fixture> = Vec::new();

    // --- Sony DualShock 4 & INZONE ---
    {
        let mut ds4 = vec![0_u8; 63];
        ds4[29] = 0x15; // cable + level 5 -> 50% charging
        fixtures.push(make_fixture(
            "sony.dualshock4",
            ep("ds4", 0x054C, 0x09CC, Some(1), Some(5), None, None),
            vec![step_read(0x01, &ds4)],
            expect_pct(50, ChargeState::Charging, None),
        ));

        let mut req = padded(
            &[
                0x0C, 0x01, 0x00, 0xFC, 0x08, 0x96, 0xC3, 0x41, 0x04, 0x01, 0x02, 0x00,
            ],
            63,
        );
        req[12] = sum8(&req[5..=11]);
        let mut resp = vec![0_u8; 63];
        // b[0]=0x02 (rid), so resp[i] == b[i+1]
        resp[0] = 14; // len = 14 -> payload b[13..=14] = resp[12..=13], csum at b[15] = resp[14]
        resp[1] = 0x04;
        resp[2] = 0xFF;
        resp[4] = 0x00;
        resp[5] = 0x96;
        resp[6] = 0xC3;
        resp[7] = 0x10;
        resp[8] = 0x04;
        resp[9] = 0x10;
        resp[10] = 0x02;
        resp[11] = 0x00;
        resp[12] = 0x01; // charging
        resp[13] = 85; // 85%
        resp[14] = sum8(&resp[4..=13]);
        fixtures.push(make_fixture(
            "sony.inzone",
            ep("inzone", 0x054C, 0x0FA8, Some(0xFF04), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x02, &req),
                step_read(0x02, &resp),
            ],
            expect_pct(85, ChargeState::Charging, None),
        ));
    }

    // --- Gamepads ---
    {
        // Switch Pro
        let mut sp = vec![0_u8; 48];
        sp[1] = (3 << 5) | 0x10; // Medium + charging
        fixtures.push(make_fixture(
            "nintendo.switch_pro",
            ep("switch_pro", 0x057E, 0x2009, Some(1), Some(5), None, None),
            vec![step_read(0x30, &sp)],
            Expectation::Reading {
                percentage: None,
                level: Some(BatteryLevel::Medium),
                voltage_mv: None,
                charge_state: ChargeState::Charging,
                reachability: Reachability::Online,
                precision: Precision::DiscreteLevel,
            },
        ));

        // Steam Controller modern
        let mut sc = vec![0_u8; 63];
        sc[0] = 0x43;
        sc[1] = 0x02;
        sc[2] = 72;
        fixtures.push(make_fixture(
            "valve.steam_controller",
            ep(
                "steam_ctrl",
                0x28DE,
                0x1304,
                Some(0xFF00),
                Some(1),
                None,
                None,
            ),
            vec![step_read(0x00, &sc)],
            expect_pct(72, ChargeState::Charging, None),
        ));

        // GameSir Nova
        let mut nova_resp = vec![0_u8; 32];
        nova_resp[0] = 0x01;
        nova_resp[18] = 88;
        fixtures.push(make_fixture(
            "gamesir.nova",
            ep("gs_nova", 0x3537, 0x1098, Some(0xFF7A), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x01, 0x01], 63)),
                step_read(0x00, &nova_resp),
            ],
            expect_pct(88, ChargeState::Discharging, None),
        ));

        // GameSir Cyclone 2
        let mut cyc = vec![0_u8; 63];
        cyc[34] = 0x01; // frame[35] charging
        cyc[35] = 64; // frame[36] pct
        fixtures.push(make_fixture(
            "gamesir.cyclone2",
            ep(
                "gs_cyc2",
                0x3537,
                0x100B,
                Some(0xFFF0),
                Some(0x0040),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x0F, &padded(&[0x03], 63)),
                step_read(0x12, &cyc),
            ],
            expect_pct(64, ChargeState::Charging, None),
        ));

        // GameSir SAPJoy
        let mut sj = vec![0_u8; 63];
        sj[0] = 14; // frame[1]
        sj[1] = 0x0A; // frame[2]
        sj[2] = 0x01; // frame[3]
        sj[12] = 77; // frame[13]
        fixtures.push(make_fixture(
            "gamesir.sapjoy",
            ep(
                "gs_sapjoy",
                0x3537,
                0x103E,
                Some(0xFFA0),
                Some(1),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x07, &padded(&[0x03, 0x0A, 0x01], 31)),
                step_read(0x06, &sj),
            ],
            expect_pct(77, ChargeState::Discharging, None),
        ));

        // Flydigi Vader 5 Pro
        let mut fd = vec![0_u8; 32];
        fd[..3].copy_from_slice(&[0x5A, 0xA5, 0x01]);
        fd[5] = 0x82;
        fd[11] = 0x14; // state=1 charging, level=4 -> 80%
        fixtures.push(make_fixture(
            "flydigi.controller",
            ep("flydigi", 0x37D7, 0x2401, Some(0xFFA0), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x5A, 0xA5, 0x01, 0x02, 0x00], 31)),
                step_read(0x00, &fd),
            ],
            expect_pct(80, ChargeState::Charging, None),
        ));

        // 8BitDo Vendor
        let mut eb_link = vec![0_u8; 63];
        eb_link[0] = 0x04;
        eb_link[3] = 0x20;
        eb_link[4] = 0x01;
        eb_link[17] = 0x01; // frame[18] linked
        let mut eb_bat = vec![0_u8; 63];
        eb_bat[0] = 0x04;
        eb_bat[3] = 0x21;
        eb_bat[4] = 0x01;
        eb_bat[17] = 0x80 | 66; // charging 66%
        fixtures.push(make_fixture(
            "eightbitdo.vendor",
            ep(
                "8bitdo_v",
                0x2DC8,
                0x310B,
                Some(0xFF7A),
                Some(1),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x81, &padded(&[0x04, 0x20, 0x01], 63)),
                step_read(0x02, &eb_link),
                step_write(0x81, &padded(&[0x04, 0x21, 0x01, 0x00, 0x00, 0x06], 63)),
                step_read(0x02, &eb_bat),
            ],
            expect_pct(66, ChargeState::Charging, None),
        ));

        // 8BitDo DInput
        let mut ebd = vec![0_u8; 34];
        ebd[13] = 92; // frame[14]
        fixtures.push(make_fixture(
            "eightbitdo.dinput",
            ep("8bitdo_d", 0x2DC8, 0x6012, Some(1), Some(5), None, None),
            vec![step_read(0x01, &ebd)],
            expect_pct(92, ChargeState::Discharging, None),
        ));

        // BEITONG KP20
        const SBOX: [u8; 16] = [
            0x0, 0xA, 0x6, 0xF, 0x1, 0xC, 0x2, 0x9, 0xE, 0x3, 0x7, 0xD, 0x4, 0xB, 0x5, 0x8,
        ];
        let enc = |x: u8| (SBOX[usize::from(x >> 4)] << 4) | ((x & 0x0F) ^ 0x08);
        fixtures.push(make_fixture(
            "beitong.controller",
            ep("beitong", 0x20BC, 0x5128, Some(0x00FF), Some(3), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x02, &padded(&[enc(0x15)], 63)),
                step_read(0x02, &[enc(0x15), enc(83), 0x00, enc(0x01)]),
            ],
            expect_pct(83, ChargeState::Charging, None),
        ));
    }

    // --- SteelSeries ---
    {
        fixtures.push(make_fixture(
            "steelseries.arctis1",
            ep(
                "ss_a1",
                0x1038,
                0x12B3,
                Some(0xFF43),
                Some(0x0202),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x06, &padded(&[0x12], 31)),
                step_read(0x06, &[0x12, 0x03, 78]),
            ],
            expect_pct(78, ChargeState::Unknown, None),
        ));
        fixtures.push(make_fixture(
            "steelseries.arctis7",
            ep(
                "ss_a7",
                0x1038,
                0x1260,
                Some(0xFF43),
                Some(0x0202),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x06, &padded(&[0x18], 31)),
                step_read(0x06, &[0x18, 65]),
            ],
            expect_pct(65, ChargeState::Unknown, None),
        ));
        fixtures.push(make_fixture(
            "steelseries.arctis9",
            ep(
                "ss_a9",
                0x1038,
                0x12C2,
                Some(0xFF00),
                Some(1),
                Some(0),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x20], 31)),
                step_read(0x00, &[0x20, 0x00, 0x00, 0x9A, 0x01]),
            ],
            expect_pct(100, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "steelseries.arctis_pro_wireless",
            ep(
                "ss_apw",
                0x1038,
                0x1290,
                Some(0xFF00),
                Some(1),
                Some(0),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x41, 0xAA], 31)),
                step_read(0x00, &[0x04]),
                step_write(0x00, &padded(&[0x40, 0xAA], 31)),
                step_read(0x00, &[0x03]),
            ],
            expect_pct(75, ChargeState::Unknown, None),
        ));
        let mut npw = vec![0_u8; 63];
        npw[5] = 6; // frame[6] -> 75%
        npw[14] = 0x02; // frame[15] charging
        fixtures.push(make_fixture(
            "steelseries.nova_pro_wireless",
            ep(
                "ss_npw",
                0x1038,
                0x225D,
                Some(0xFFC0),
                Some(1),
                Some(4),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x06, &padded(&[0xB0], 63)),
                step_read(0x06, &npw),
            ],
            expect_pct(75, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "steelseries.nova_family",
            ep("ss_nf", 0x1038, 0x22A1, Some(0xFFC0), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0xB0], 63)),
                step_read(0x00, &[0xB0, 0x00, 82, 0x03, 0x00, 0x00, 0x00]),
            ],
            expect_pct(82, ChargeState::Discharging, None),
        ));
        let mut ne = vec![0_u8; 63];
        ne[0] = 0xB0;
        ne[5] = 91; // frame[6]
        ne[13] = 0x03; // frame[14]
        ne[14] = 0x00; // frame[15]
        fixtures.push(make_fixture(
            "steelseries.nova_elite",
            ep(
                "ss_ne",
                0x1038,
                0x2244,
                Some(0xFFC0),
                Some(1),
                Some(3),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x01, &padded(&[0xB0], 63)),
                step_read(0x01, &ne),
            ],
            expect_pct(91, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "steelseries.aerox",
            ep(
                "ss_aerox",
                0x1038,
                0x1838,
                Some(0xFF00),
                Some(1),
                Some(3),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0xD2], 63)),
                step_read(0x00, &[0xD2, 17]), // (17-1)*5 = 80%
            ],
            expect_pct(80, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "steelseries.rival",
            ep(
                "ss_rival",
                0x1038,
                0x1830,
                Some(0xFF00),
                Some(1),
                Some(3),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0xAA, 0x01], 63)),
                step_read(0x00, &[55, 0x00, 0x01]),
            ],
            expect_pct(55, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "steelseries.rival_pro",
            ep(
                "ss_rp",
                0x1038,
                0x1880,
                Some(0xFF00),
                Some(1),
                Some(2),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x92], 63)),
                step_read(0x00, &[0x92, 0x00, 68]),
            ],
            expect_pct(68, ChargeState::Unknown, None),
        ));
    }

    // --- Corsair ---
    {
        fixtures.push(make_fixture(
            "corsair.void",
            ep(
                "cor_void",
                0x1B1C,
                0x0A55,
                Some(0xFFC5),
                Some(1),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0xC9, &padded(&[0x64], 63)),
                step_read(0xCA, &[0x00, 0x80 | 74, 0x00, 0x01]),
            ],
            expect_pct(74, ChargeState::Discharging, None),
        ));
        let mut br_bat = vec![0_u8; 63];
        br_bat[0] = 0x01; // source
        br_bat[1] = 0x02;
        br_bat[2] = 0x00; // OK
        br_bat[3..7].copy_from_slice(&840_u32.to_le_bytes()); // 84%
        let mut br_chg = vec![0_u8; 63];
        br_chg[0] = 0x01;
        br_chg[1] = 0x02;
        br_chg[2] = 0x00;
        br_chg[3..7].copy_from_slice(&1_u32.to_le_bytes()); // charging
        fixtures.push(make_fixture(
            "corsair.bragi",
            ep(
                "cor_bragi",
                0x1B1C,
                0x2B00,
                Some(0xFF42),
                Some(1),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x02, &padded(&[0x09, 0x02, 0x0F], 63)),
                step_read(0x00, &br_bat),
                step_flush_timeout(),
                step_write(0x02, &padded(&[0x09, 0x02, 0x10], 63)),
                step_read(0x00, &br_chg),
            ],
            expect_pct(84, ChargeState::Charging, None),
        ));
    }

    // --- HyperX ---
    {
        fixtures.push(make_fixture(
            "hyperx.cloud3",
            ep("hx_c3", 0x03F0, 0x05B7, Some(0xFF90), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x66, &padded(&[0x82], 61)),
                step_read(0x66, &[0x82, 0x01, 0x00, 0x00]),
                step_flush_timeout(),
                step_write(0x66, &padded(&[0x8A], 61)),
                step_read(0x66, &[0x8A, 0x00, 0x00, 0x00]),
                step_flush_timeout(),
                step_write(0x66, &padded(&[0x89], 61)),
                step_read(0x66, &[0x89, 0x0F, 0xA0, 90]), // 4000mV, 90%
            ],
            expect_pct(90, ChargeState::Discharging, Some(4000)),
        ));
        fixtures.push(make_fixture(
            "hyperx.cloud3_s",
            ep("hx_c3s", 0x03F0, 0x06BE, Some(0x01C0), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x0C, &padded(&[0x02, 0x03, 0x01, 0x00, 0x02], 63)),
                step_read(0x0C, &[0x02, 0x03, 0x01, 0x00, 0x02, 0x02]),
                step_flush_timeout(),
                step_write(0x0C, &padded(&[0x02, 0x03, 0x01, 0x00, 0x06], 63)),
                step_read(0x0C, &[0x02, 0x03, 0x01, 0x00, 0x06, 77]),
                step_flush_timeout(),
                step_write(0x0C, &padded(&[0x02, 0x03, 0x01, 0x00, 0x48], 63)),
                step_read(0x0C, &[0x02, 0x03, 0x01, 0x00, 0x48, 0x01]),
            ],
            expect_pct(77, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "hyperx.cloud2",
            ep(
                "hx_c2",
                0x03F0,
                0x0696,
                Some(0xFF90),
                Some(0x0303),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x06, &padded(&[0xFF, 0xBB, 0x02], 63)),
                step_read(0x06, &[0xFF, 0xBB, 0x02, 0x00, 0x0F, 0xA0, 81]),
                step_flush_timeout(),
                step_write(0x06, &padded(&[0xFF, 0xBB, 0x03], 63)),
                step_read(0x06, &[0xFF, 0xBB, 0x03, 0x01, 0x00, 0x00, 0x00]),
            ],
            expect_pct(81, ChargeState::Charging, Some(4000)),
        ));
        fixtures.push(make_fixture(
            "hyperx.cloud2_kingston",
            ep("hx_c2k", 0x0951, 0x1718, Some(0xFF13), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(
                    0x06,
                    &padded(
                        &[
                            0x00, 0x02, 0x00, 0x9A, 0x00, 0x00, 0x68, 0x4A, 0x8E, 0x0A, 0x00, 0x00,
                            0x00, 0xBB, 0x02, 0x00,
                        ],
                        61,
                    ),
                ),
                step_read(0x0B, &[0x00, 0xBB, 0x02, 0x00, 0x00, 0x00, 62]),
                step_flush_timeout(),
                step_write(
                    0x06,
                    &padded(
                        &[
                            0x00, 0x02, 0x00, 0x9A, 0x00, 0x00, 0x68, 0x4A, 0x8E, 0x0A, 0x00, 0x00,
                            0x00, 0xBB, 0x03, 0x00,
                        ],
                        61,
                    ),
                ),
                step_read(0x0B, &[0x00, 0xBB, 0x03, 0x00, 0x00, 0x00, 0x00]),
            ],
            expect_pct(62, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "hyperx.cloud_alpha",
            ep("hx_ca", 0x03F0, 0x098D, Some(0xFF90), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x21, &padded(&[0xBB, 0x03], 31)),
                step_read(0x21, &[0xBB, 0x03, 0x02]),
                step_flush_timeout(),
                step_write(0x21, &padded(&[0xBB, 0x0C], 31)),
                step_read(0x21, &[0xBB, 0x0C, 0x00]),
                step_flush_timeout(),
                step_write(0x21, &padded(&[0xBB, 0x0B], 31)),
                step_read(0x21, &[0xBB, 0x0B, 95]),
            ],
            expect_pct(95, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "hyperx.alpha2_haste",
            ep(
                "hx_haste",
                0x03F0,
                0x0AB5,
                Some(0xFF13),
                Some(1),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x52, &padded(&[0x01], 31)),
                step_read(0x53, &[0x01, 0x01, 0, 0, 0, 0, 0]),
                step_flush_timeout(),
                step_write(0x50, &padded(&[0x02], 31)),
                step_read(0x51, &[0x02, 71, 0x01, 0, 0, 0xA0, 0x0F]),
            ],
            expect_pct(71, ChargeState::Charging, Some(4000)),
        ));
        fixtures.push(make_fixture(
            "hyperx.cloud_flight",
            ep("hx_cf", 0x0951, 0x16C4, Some(0xFF90), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x21, &padded(&[0xFF, 0x05], 19)),
                step_read(0x21, &[0xFF, 0x05, 0x0F, 0xA0]), // 4000mV -> 100%
            ],
            expect_pct(100, ChargeState::Discharging, Some(4000)),
        ));
        let mut sc_conn = vec![0_u8; 31];
        sc_conn[10] = 0x01; // frame[11]
        let mut sc_bat = vec![0_u8; 31];
        sc_bat[0] = 0x12;
        sc_bat[1] = 0x05;
        sc_bat[8] = 0x03; // frame[9] discharging
        sc_bat[10..12].copy_from_slice(&4042_u16.to_le_bytes()); // 90%
        fixtures.push(make_fixture(
            "hyperx.stinger_core",
            ep("hx_sc", 0x03F0, 0x0A93, Some(0xFF00), Some(1), None, None),
            vec![
                step_send_feature(
                    0xFF,
                    &padded(
                        &[
                            0x07, 0x00, 0xFD, 0x04, 0x00, 0x00, 0x02, 0x82, 0x03, 0x05, 0x29, 0x0F,
                            0x00, 0x00, 0x03, 0x68, 0x10, 0x00, 0x00,
                        ],
                        31,
                    ),
                ),
                step_get_feature(0xFF, 31, &sc_conn),
                step_send_feature(
                    0xFF,
                    &padded(
                        &[
                            0x07, 0x00, 0xFD, 0x04, 0x0C, 0xF1, 0x02, 0x01, 0x04, 0xF0, 0x0C,
                        ],
                        31,
                    ),
                ),
                step_get_feature(0xFF, 31, &sc_bat),
            ],
            expect_pct(90, ChargeState::Discharging, Some(4042)),
        ));
    }

    // --- Headsets (Airoha RACE, Jabra, JBL, Plantronics, Fractal, Alienware, Moondrop) ---
    {
        let airoha_req = padded(&[0x07, 0x80, 0x05, 0x5A, 0x03, 0x00, 0xD6, 0x0C], 61);
        let mut audeze_resp = vec![0_u8; 61];
        audeze_resp[4..9].copy_from_slice(&[0xD6, 0x0C, 0x00, 0x00, 88]);
        fixtures.push(make_fixture(
            "audeze.maxwell",
            ep("audeze", 0x3329, 0x4B19, Some(0xFF13), Some(1), None, None),
            vec![
                step_write(0x06, &airoha_req),
                step_get_input(0x07, 61, &audeze_resp),
            ],
            expect_pct(88, ChargeState::Unknown, None),
        ));

        let mut endorfy_resp = vec![0_u8; 61];
        // frame[0]=0x07 -> resp[i] == frame[i+1], frame[3..] == resp[2..]
        endorfy_resp[2..11].copy_from_slice(&[0x05, 0x5D, 0x05, 0x00, 0xD6, 0x0C, 0x00, 0x00, 67]);
        fixtures.push(make_fixture(
            "endorfy.viro",
            ep("endorfy", 0x3738, 0x0072, Some(0xFF13), Some(1), None, None),
            vec![
                step_write(0x06, &airoha_req),
                step_get_input(0x07, 61, &endorfy_resp),
            ],
            expect_pct(67, ChargeState::Unknown, None),
        ));

        let mut akg_resp = vec![0_u8; 61];
        akg_resp[0] = 9; // frame[1] length = 9
        akg_resp[2..11].copy_from_slice(&[0x05, 0x5D, 0x05, 0x00, 0xD6, 0x0C, 0x00, 0x01, 79]);
        fixtures.push(make_fixture(
            "akg.n9_hybrid",
            ep("akg", 0x0ECB, 0x20D9, Some(0xFF13), Some(1), None, None),
            vec![
                step_write(0x06, &airoha_req),
                step_get_input(0x07, 61, &akg_resp),
            ],
            expect_pct(79, ChargeState::Unknown, None),
        ));

        let gsi = br#"{"GSI":{"240":"84"}}"#;
        let mut tb_resp = vec![0_u8; 61];
        tb_resp[0] = gsi.len() as u8; // frame[1]
        tb_resp[2..2 + gsi.len()].copy_from_slice(gsi);
        fixtures.push(make_fixture(
            "turtlebeach.stealth700",
            ep("tb700", 0x10F5, 0x2251, Some(0xFF13), Some(1), None, None),
            vec![
                step_write(
                    0x06,
                    &padded(&[0x06, 0x00, 0x05, 0x5A, 0x02, 0x00, 0x42, 0x99], 61),
                ),
                step_write(
                    0x06,
                    &padded(
                        &[
                            0x0E, 0x00, 0x05, 0x5A, 0x0A, 0x00, 0x02, 0x99, 0x40, 0x01, 0x00, 0x01,
                            0xB3, b'G', b'S', b'I',
                        ],
                        61,
                    ),
                ),
                step_get_input(0x07, 61, &tb_resp),
            ],
            expect_pct(84, ChargeState::Unknown, None),
        ));

        fixtures.push(make_fixture(
            "jabra.gnp",
            ep(
                "jabra",
                0x0B0E,
                0x2E51,
                Some(0xFF00),
                Some(5),
                None,
                Some("Jabra Link 390"),
            ),
            vec![
                step_flush_timeout(),
                step_write(0x05, &padded(&[0x04, 0x00, 0x20, 0x46, 0x12, 0x02], 63)),
                step_read(0x05, &[0x00, 0x04, 0x20, 0xC8, 0x12, 0x02, 0x01, 93]),
            ],
            expect_pct(93, ChargeState::Charging, None),
        ));

        fixtures.push(make_fixture(
            "jbl.hidv1",
            ep("jbl1", 0x0ECB, 0x206B, Some(1), Some(0), Some(3), None),
            vec![
                step_get_feature(0x68, 1, &[0x01]),
                step_get_feature(0x49, 1, &[70]),
            ],
            expect_pct(70, ChargeState::Unknown, None),
        ));

        fixtures.push(make_fixture(
            "jbl.hidv3",
            ep("jbl3", 0x0ECB, 0x2125, Some(0xFF12), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_send_feature(
                    0x88,
                    &[0x00, 0xDD, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x0D, 0x00],
                ),
                step_read(
                    0x86,
                    &[
                        0x00,
                        0xDD,
                        0x01,
                        0x00,
                        0x01,
                        0,
                        0,
                        0,
                        0,
                        0x00,
                        0x0D,
                        0x00,
                        0x01,
                        0x00,
                        0x80 | 61,
                    ],
                ),
            ],
            expect_pct(61, ChargeState::Charging, None),
        ));

        fixtures.push(make_fixture(
            "plantronics.bt600",
            ep("bt600", 0x047F, 0x02EE, Some(0xFFA2), Some(3), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x06, &[0x01]),
                step_write(
                    0x07,
                    &padded(
                        &[0x01, 0x01, 0x10, 0x07, 0x20, 0x00, 0x00, 0x01, 0x01, 0x02],
                        63,
                    ),
                ),
                step_read(
                    0x07,
                    &[0x01, 0x01, 0x10, 0x07, 0x02, 0x00, 0x00, 0x08, 0x01, 0x02],
                ),
                step_write(
                    0x07,
                    &padded(
                        &[0x01, 0x01, 0x10, 0x06, 0x20, 0x00, 0x00, 0x02, 0x0A, 0x1A],
                        63,
                    ),
                ),
                step_read(
                    0x07,
                    &[
                        0x01, 0x01, 0x10, 0x06, 0x02, 0x00, 0x00, 0x03, 0x0A, 0x1A, 0x04, 0x06,
                        0x01,
                    ],
                ),
            ],
            expect_pct(80, ChargeState::Charging, None),
        ));

        let mut fd_dongle = vec![0_u8; 23];
        fd_dongle[0] = 0x11;
        fd_dongle[1] = 0x21;
        fd_dongle[3] = 1; // frame[4] linked
        let mut fd_hs = vec![0_u8; 23];
        fd_hs[0] = 0xF1;
        fd_hs[1] = 0x21;
        fd_hs[14] = 89; // frame[15]
        fd_hs[22] = 0; // frame[23] discharging
        fixtures.push(make_fixture(
            "fractal.scape",
            ep("fractal", 0x36BC, 0x0001, Some(0xFF00), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x02, &padded(&[0x11, 0x21], 63)),
                step_read(0x02, &fd_dongle),
                step_flush_timeout(),
                step_write(0x02, &padded(&[0xF1, 0x21], 63)),
                step_read(0x02, &fd_hs),
            ],
            expect_pct(89, ChargeState::Discharging, None),
        ));

        let mut aw = vec![0_u8; 61];
        aw[0] = 0xC0;
        aw[1] = 0x0A;
        aw[2] = 0x09; // frame[3]
        aw[5] = 73; // frame[6]
        aw[6] = 1; // frame[7] charging
        aw[12] = aw[2..=11].iter().fold(0_u8, |a, b| a ^ b); // frame[13]
        fixtures.push(make_fixture(
            "alienware.aw920h",
            ep("aw920h", 0x413C, 0xA515, Some(0xFF13), Some(1), None, None),
            vec![
                step_write(0x06, &padded(&[0xC0, 0x0A], 61)),
                step_get_input(0x07, 61, &aw),
            ],
            expect_pct(73, ChargeState::Charging, None),
        ));

        fixtures.push(make_fixture(
            "moondrop.gm01_pro",
            ep(
                "moondrop",
                0x35D8,
                0x0133,
                Some(0x000C),
                Some(1),
                Some(3),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x4B, &padded(&[0x80, 0x27, 0x00], 63)),
                step_read(0x4B, &[0x80, 0x27, 0x06, 0, 0, 0, 0, 0x01, 58]),
            ],
            expect_pct(58, ChargeState::Unknown, None),
        ));
    }

    // --- Logitech G435 & ASTRO ---
    {
        let g435_pkt = |cmd: u8, body: &[u8]| {
            let mut p = vec![0_u8; 64];
            p[0] = cmd;
            p[1] = body.len() as u8;
            p[4..4 + body.len()].copy_from_slice(body);
            let c = crc16(&p, 0x0000);
            p[2] = (c & 0xFF) as u8;
            p[3] = (c >> 8) as u8;
            p
        };
        let mut g435_resp = vec![0_u8; 64];
        g435_resp[10..18].copy_from_slice(&[0x00, 0x07, 0x00, 0x06, 0x05, 0x00, 0xA0, 0x0F]); // 4000mV -> 80%
        fixtures.push(make_fixture(
            "logitech.g435",
            ep("g435", 0x046D, 0x0ADF, Some(0xFF00), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &g435_pkt(0x84, &[0x01])),
                step_write(0x00, &g435_pkt(0x85, &[0x01])),
                step_write(
                    0x00,
                    &g435_pkt(
                        0x80,
                        &[
                            0x50, 0x41, 0x06, 0x01, 0x0D, 0x00, 0x25, 0x34, 0x12, 0x5A, 0x5A, 0x01,
                            0x00, 0x00, 0x00, 0xF0, 0x00, 0x00, 0x00,
                        ],
                    ),
                ),
                step_write(0x00, &g435_pkt(0x80, &[0x50, 0x41, 0x0E, 0x02, 0x01, 0xE0])),
                step_write(
                    0x00,
                    &g435_pkt(0x80, &[0x50, 0x41, 0x0E, 0x03, 0x02, 0xE1, 0x01]),
                ),
                step_write(0x00, &g435_pkt(0x80, &[0x50, 0x41, 0x0E, 0x04, 0x01, 0xE0])),
                step_write(
                    0x00,
                    &g435_pkt(0x80, &[0x50, 0x41, 0x06, 0x05, 0x01, 0x00, 0x31]),
                ),
                step_read(0x00, &g435_resp),
                step_write(0x00, &g435_pkt(0x85, &[0x02])),
            ],
            expect_pct(80, ChargeState::Unknown, Some(4000)),
        ));

        fixtures.push(make_fixture(
            "astro.a50_modern",
            ep(
                "astro_m",
                0x046D,
                0x0B0B,
                Some(0xFF32),
                Some(0x0074),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x02, &padded(&[0x0C, 0x03, 0x00, 0x06, 0x0C], 63)),
                step_read(0x02, &[0x0C, 0x06, 0x00, 0x06, 0x0C, 86, 0x00, 0x01]),
            ],
            expect_pct(86, ChargeState::Charging, None),
        ));

        fixtures.push(make_fixture(
            "astro.legacy",
            ep("astro_l", 0x9886, 0x002C, Some(0xFF32), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x02, &padded(&[0x54], 63)),
                step_read(0x02, &[0x02, 0x01, 0x02]), // connected, not docked
                step_flush_timeout(),
                step_write(0x02, &padded(&[0x7C], 63)),
                step_read(0x02, &[0x02, 0x01, 69]),
            ],
            expect_pct(69, ChargeState::Discharging, None),
        ));
    }

    // --- Razer Headsets & ASUS ROG ---
    {
        let bs_req = |cmd: u8, seq: u8| {
            let mut r = [0_u8; 64];
            r[0] = 0x02;
            r[2] = seq;
            r[6] = 0x04;
            r[9] = 0x80;
            r[10] = cmd;
            r[62] = r[..62].iter().fold(0_u8, |a, b| a ^ b);
            r[1..].to_vec()
        };
        let bs_resp = |cmd: u8, seq: u8, val: u8| {
            let mut r = [0_u8; 64];
            r[0] = 0x02;
            r[2] = seq;
            r[10] = cmd;
            r[12] = 1;
            r[13] = val;
            r[62] = r[1..62].iter().fold(0_u8, |a, b| a ^ b);
            r[1..].to_vec()
        };
        fixtures.push(make_fixture(
            "razer.blackshark_v2_hs",
            ep("rz_v2hs", 0x1532, 0x0565, Some(0xFF14), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x02, &bs_req(0x21, 0x60)),
                step_read(0x02, &bs_resp(0x21, 0x60, 76)),
                step_flush_timeout(),
                step_write(0x02, &bs_req(0x2A, 0x61)),
                step_read(0x02, &bs_resp(0x2A, 0x61, 0)),
            ],
            expect_pct(76, ChargeState::Discharging, None),
        ));

        let mut bs2020 = vec![0_u8; 63];
        bs2020[..8].copy_from_slice(&[0x0F, 0x05, 0xFE, 0x12, 0x04, 0x1F, 0x08, 0x05]);
        bs2020[10] = 0x00; // frame[11] discharging
        bs2020[11..13].copy_from_slice(&4000_u16.to_be_bytes());
        bs2020[13] = 63; // frame[14]
        fixtures.push(make_fixture(
            "razer.blackshark_v2_pro_2020",
            ep(
                "rz_v2_2020",
                0x1532,
                0x0528,
                Some(0xFF00),
                Some(1),
                None,
                None,
            ),
            vec![
                step_send_feature(
                    0xFF,
                    &padded(&[0x0A, 0x00, 0xFD, 0x04, 0x12, 0xF1, 0x02, 0x05], 63),
                ),
                step_get_feature(0xFF, 63, &bs2020),
            ],
            expect_pct(63, ChargeState::Discharging, Some(4000)),
        ));

        let v3x_req = |cmd: u8, txid: u8| {
            let mut r = [0_u8; 64];
            r[0] = 0x07;
            r[2] = txid;
            r[6] = 0x02;
            r[7] = 0x07;
            r[8] = cmd;
            r[62] = r[3..=60].iter().fold(0_u8, |a, b| a ^ b);
            r[1..].to_vec()
        };
        let v3x_resp = |cmd: u8, txid: u8, val: u8| {
            let mut r = vec![0_u8; 63];
            r[0] = 0x02; // frame[1] ok
            r[1] = txid;
            r[5] = 2;
            r[6] = 0x07;
            r[7] = cmd;
            r[9] = val; // frame[10]
            r
        };
        fixtures.push(make_fixture(
            "razer.blackshark_v3_x",
            ep(
                "rz_v3x",
                0x1532,
                0x057D,
                Some(0xFF00),
                Some(1),
                Some(0),
                None,
            ),
            vec![
                step_send_feature(0x07, &v3x_req(0x80, 0x41)),
                step_get_feature(0x07, 63, &v3x_resp(0x80, 0x41, 88)),
                step_send_feature(0x07, &v3x_req(0x84, 0x42)),
                step_get_feature(0x07, 63, &v3x_resp(0x84, 0x42, 1)),
            ],
            expect_pct(88, ChargeState::Charging, None),
        ));

        let v3_pkt = |dir: u8, cls: u8| {
            let mut r = [0_u8; 64];
            r[0] = 0x02;
            r[2] = 0x60;
            r[6] = 0x04;
            r[9] = dir;
            r[10] = cls;
            r[62] = r[..62].iter().fold(0_u8, |a, b| a ^ b);
            r[1..].to_vec()
        };
        let mut v3_bat = vec![0_u8; 63];
        v3_bat[1] = 0x60;
        v3_bat[9] = 0x21;
        v3_bat[12] = 54; // frame[13]
        v3_bat[13] = 1; // frame[14] charging
        fixtures.push(make_fixture(
            "razer.blackshark_v3",
            ep(
                "rz_v3",
                0x1532,
                0x0577,
                Some(0xFF00),
                Some(1),
                Some(5),
                None,
            ),
            vec![
                step_write(0x02, &v3_pkt(0x00, 0x02)),
                step_write(0x02, &v3_pkt(0x00, 0x2A)),
                step_write(0x02, &v3_pkt(0x80, 0x2A)),
                step_flush_timeout(),
                step_write(0x02, &v3_pkt(0x80, 0x21)),
                step_read(0x02, &v3_bat),
            ],
            expect_pct(54, ChargeState::Charging, None),
        ));

        let mut bp_bat = vec![0_u8; 63];
        bp_bat[12] = 0x21;
        bp_bat[13] = 0x01;
        bp_bat[14] = 0x01;
        bp_bat[15] = 81;
        let mut bp_chg = vec![0_u8; 63];
        bp_chg[12] = 0x2A;
        bp_chg[13] = 0x01;
        bp_chg[14] = 0x01;
        bp_chg[15] = 0;
        fixtures.push(make_fixture(
            "razer.barracuda_pro",
            ep("rz_bp", 0x1532, 0x053A, Some(0xFF00), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(
                    0x01,
                    &padded(&[0x80, 0x07, b'P', b'A', 0x0E, 0x08, 0x02, 0xE1, 0x01], 63),
                ),
                step_write(
                    0x01,
                    &padded(&[0x80, 0x08, b'P', b'A', 0x08, 0x08, 0x03, 0x21], 63),
                ),
                step_read(0x01, &bp_bat),
                step_write(
                    0x01,
                    &padded(&[0x80, 0x08, b'P', b'A', 0x08, 0x08, 0x03, 0x2A], 63),
                ),
                step_read(0x01, &bp_chg),
                step_write(
                    0x01,
                    &padded(&[0x80, 0x07, b'P', b'A', 0x0E, 0x08, 0x02, 0xE1, 0x00], 63),
                ),
            ],
            expect_pct(81, ChargeState::Discharging, None),
        ));

        // Barracuda X (Macronix 0x0552)
        let bx_pi = |group: u8, payload: &[u8]| {
            let mut msg = vec![0_u8; 10 + payload.len()];
            msg[0] = 0x50;
            msg[1] = 0x49;
            msg[2] = group;
            msg[8..10].copy_from_slice(&(payload.len() as u16).to_le_bytes());
            msg[10..].copy_from_slice(payload);
            let mut r = vec![0_u8; 63];
            r[0] = 0x80;
            r[1] = msg.len() as u8;
            r[2..2 + msg.len()].copy_from_slice(&msg);
            r
        };
        fixtures.push(make_fixture(
            "razer.barracuda_x",
            ep("rz_bx", 0x1532, 0x0552, Some(0xFF00), Some(0), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x01, &padded(&[0x40], 63)),
                step_read(0x01, &[0x40, 0x01, 0x01]),
                step_write(
                    0x01,
                    &padded(&[0x80, 0x07, 0x50, 0x41, 0x0E, 0x01, 0x02, 0xE1, 0x01], 63),
                ),
                step_read(0x01, &bx_pi(0x0E, &[0x0E, 0x81, 0x00])),
                step_write(
                    0x01,
                    &padded(
                        &[0x80, 0x08, 0x50, 0x41, 0x08, 0x02, 0x03, 0x21, 0x00, 0x00],
                        63,
                    ),
                ),
                step_read(0x01, &bx_pi(0x08, &[0x21, 0x01, 0x01, 68])),
                step_write(
                    0x01,
                    &padded(
                        &[0x80, 0x08, 0x50, 0x41, 0x08, 0x03, 0x03, 0x2A, 0x00, 0x00],
                        63,
                    ),
                ),
                step_read(0x01, &bx_pi(0x08, &[0x2A, 0x01, 0x01, 0x00])),
                step_write(
                    0x01,
                    &padded(&[0x80, 0x07, 0x50, 0x41, 0x0E, 0x04, 0x02, 0xE1, 0x00], 63),
                ),
            ],
            expect_pct(68, ChargeState::Discharging, None),
        ));

        // ASUS ROG Cetra & ROG Peripheral
        fixtures.push(make_fixture(
            "asus.rog_cetra",
            ep(
                "rog_cetra",
                0x0B05,
                0x1CAC,
                Some(0xFF00),
                Some(1),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0xCC, &padded(&[0x12, 0x12], 63)),
                step_read(0xCC, &[0x12, 0x12, 0, 0, 90, 75, 100]),
            ],
            expect_pct(75, ChargeState::Unknown, None),
        ));
        let mut rog_kb = vec![0_u8; 63];
        rog_kb[0] = 0x12;
        rog_kb[1] = 0x01;
        rog_kb[5] = 88; // frame[6]
        rog_kb[8] = 1; // frame[9] charging
        fixtures.push(make_fixture(
            "asus.rog_peripheral",
            ep(
                "rog_azoth",
                0x0B05,
                0x1A85,
                Some(0xFF00),
                Some(1),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x02, &padded(&[0x12, 0x01], 63)),
                step_read(0x02, &rog_kb),
            ],
            expect_pct(88, ChargeState::Charging, None),
        ));
    }

    // --- DriverCore 0x83 + Nordic 0x05 + Pulsar + VAXEE + Endgame ---
    {
        let dc_req = padded(&[0x00, 0x00, 0x02, 0x02, 0x00, 0x83], 64);
        let dc_resp = padded(&[0xA1, 0x00, 0x02, 0x02, 0x00, 0x83, 0x00, 77], 64);
        for (pid_id, vid, pid) in [
            ("glorious.mouse", 0x258A_u16, 0x2022_u16),
            ("hitscan.mouse", 0x3770, 0x0136),
            ("gwolves.mouse", 0x33E4, 0x3617),
            ("arbiter.akitsu", 0x373E, 0x000A),
            ("lamzu.mouse", 0x37B0, 0x0013),
            ("wlmouse.mouse", 0x36A7, 0xA883),
            ("mchose.a5_gen1", 0x2023, 0xF013),
        ] {
            let expected_charge = if pid_id == "hitscan.mouse" {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            fixtures.push(make_fixture(
                pid_id,
                ep(pid_id, vid, pid, Some(0xFFFF), Some(0), None, None),
                vec![
                    step_send_feature(0x00, &dc_req),
                    step_get_feature(0x00, 64, &dc_resp),
                ],
                expect_pct(77, expected_charge, None),
            ));
        }

        let n05_req = padded(&[0x15, 0x00, 0x00, 0x01], 31);
        let mut n05_resp = vec![0_u8; 31];
        n05_resp[0] = 0x15;
        n05_resp[8] = 66; // frame[9]
        n05_resp[9] = 1; // frame[10] charging
        n05_resp[11] = 1; // frame[12] connected
        for (id, pid) in [("ninjutso.sora_v2", 0xAE1C_u16), ("sprime.pm1", 0xAC1C)] {
            fixtures.push(make_fixture(
                id,
                ep(id, 0x1915, pid, Some(0xFFA0), Some(1), None, None),
                vec![
                    step_send_feature(0x05, &n05_req),
                    step_get_feature(0x05, 31, &n05_resp),
                ],
                expect_pct(66, ChargeState::Charging, None),
            ));
        }

        let ten_req = |cmd: u8| padded(&[cmd, 0x00, 0x00, 0x01], 15);
        let ten_resp = |cmd: u8, v: u8| {
            let mut r = vec![0_u8; 15];
            r[0] = cmd;
            r[7] = v; // frame[8]
            r
        };
        fixtures.push(make_fixture(
            "ninjutso.ten",
            ep("n_ten", 0x093A, 0xEB01, Some(0xFF01), Some(1), None, None),
            vec![
                step_send_feature(0x06, &ten_req(0x13)),
                step_get_feature(0x06, 15, &ten_resp(0x13, 1)),
                step_send_feature(0x06, &ten_req(0x12)),
                step_get_feature(0x06, 15, &ten_resp(0x12, 85)),
                step_send_feature(0x06, &ten_req(0x11)),
                step_get_feature(0x06, 15, &ten_resp(0x11, 0)),
            ],
            expect_pct(85, ChargeState::Discharging, None),
        ));

        let mut p_req = vec![0_u8; 64];
        p_req[1] = 0x08;
        p_req[2] = 0x81;
        p_req[3] = 0x01;
        let s: u16 = p_req[..62].iter().map(|&b| u16::from(b)).sum();
        p_req[62] = (s & 0xFF) as u8;
        p_req[63] = (s >> 8) as u8;
        let mut p_resp = vec![0_u8; 64];
        p_resp[0] = 0x01;
        p_resp[1] = 0x08;
        p_resp[2] = 0x81;
        p_resp[3] = 0x01;
        p_resp[6] = 94;
        fixtures.push(make_fixture(
            "pulsar.x2_v3",
            ep("pulsar", 0x3710, 0x5402, Some(0xFF00), Some(1), None, None),
            vec![
                step_send_feature(0x00, &p_req),
                step_get_feature(0x00, 64, &p_resp),
            ],
            expect_pct(94, ChargeState::Unknown, None),
        ));

        fixtures.push(make_fixture(
            "vaxee.mouse",
            ep("vaxee", 0x3057, 0x1002, Some(0xFF05), Some(1), None, None),
            vec![
                step_send_feature(0x0E, &padded(&[0xA5, 0x0B, 0x01, 0x01], 63)),
                step_get_feature(0x0E, 63, &padded(&[0xA5, 0x0B, 0x01, 0x01, 16], 63)),
                step_send_feature(0x0E, &padded(&[0xA5, 0x10, 0x01, 0x01], 63)),
                step_get_feature(0x0E, 63, &padded(&[0xA5, 0x10, 0x01, 0x01, 0], 63)),
            ],
            expect_pct(80, ChargeState::Discharging, None),
        ));

        let mut eg_resp = vec![0_u8; 63];
        eg_resp[0] = 0x01;
        eg_resp[15] = 72; // frame[16]
        fixtures.push(make_fixture(
            "endgame.4k",
            ep("endgame", 0x3367, 0x1970, Some(0xFF01), Some(2), None, None),
            vec![
                step_send_feature(0xA1, &padded(&[0xB4], 63)),
                step_get_feature(0xA1, 63, &eg_resp),
            ],
            expect_pct(72, ChargeState::Unknown, None),
        ));
    }

    // --- CompX 0x08 (Kysona, Chimera, Cherry XTRFY, Corsair Compx, Zenith) ---
    {
        let compx_frame = |cmd: u8, b5: u8, b6: u8, b7: u8| {
            let mut f = [0_u8; 17];
            f[0] = 0x08;
            f[1] = cmd;
            f[5] = b5;
            f[6] = b6;
            f[7] = b7;
            f[8] = 0x0F;
            f[9] = 0xA0; // 4000mV
            f[16] = 0x55_u8.wrapping_sub(sum8(&f[..16]));
            f[1..].to_vec()
        };
        let req03 = padded(&[0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x4A], 16);
        let req04 = padded(&[0x04, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x49], 16);
        for (id, vid, pid) in [
            ("kysona.mouse", 0x3554_u16, 0xF55D_u16),
            ("chimera.v2", 0x3554, 0xF512),
            ("cherry.xtrfy", 0x046A, 0x0330),
            ("corsair.compx", 0x1B1C, 0x2B2A),
        ] {
            fixtures.push(make_fixture(
                id,
                ep(id, vid, pid, Some(0xFF02), Some(2), None, None),
                vec![
                    step_flush_timeout(),
                    step_write(0x08, &req03),
                    step_read(0x08, &compx_frame(0x03, 1, 1, 0)),
                    step_flush_timeout(),
                    step_write(0x08, &req04),
                    step_read(0x08, &compx_frame(0x04, 2, 85, 1)),
                ],
                expect_pct(85, ChargeState::Charging, Some(4000)),
            ));
        }

        fixtures.push(make_fixture(
            "zenith.pro",
            ep("zenith", 0x260D, 0x1114, Some(0xFF02), Some(2), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x08, &req04),
                step_read(0x08, &compx_frame(0x04, 0, 77, 0)),
            ],
            expect_pct(77, ChargeState::Discharging, Some(4000)),
        ));
    }

    // --- Enthusiast Mice & Receivers ---
    {
        fixtures.push(make_fixture(
            "atk.zero",
            ep("atk_z", 0x373B, 0x124F, Some(0xFF05), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(
                    0x08,
                    &padded(&[0x7D, 0x72, 0x02, 0x00, 0x01, 0x07, 0x01], 63),
                ),
                step_read(0x08, &[0x72, 0x00, 0x00, 0x00, 0x07, 0x01, 64]),
            ],
            expect_pct(64, ChargeState::Unknown, None),
        ));
        let mut nk_req = padded(&[0x00, 0x72, 0x02, 0x00, 0x01, 0x07, 0x01], 63);
        nk_req[0] = sum8(&nk_req[1..]);
        let mut nk_chg = padded(&[0x00, 0x72, 0x02, 0x00, 0x01, 0x0F, 0x01], 63);
        nk_chg[0] = sum8(&nk_chg[1..]);
        fixtures.push(make_fixture(
            "atk.bitmouse_nk",
            ep("atk_nk", 0x373B, 0x1216, Some(0xFF05), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x08, &nk_req),
                step_read(0x08, &[0x72, 0x00, 0x00, 0x00, 0x07, 0x01, 72]),
                step_flush_timeout(),
                step_write(0x08, &nk_chg),
                step_read(0x08, &[0x72, 0x00, 0x00, 0x00, 0x0F, 0x01, 0x01]),
            ],
            expect_pct(72, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "atk.nano_8k",
            ep("atk_n8k", 0x373B, 0x1178, Some(0xFF10), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_send_feature(0x00, &padded(&[0x82], 15)),
                step_read(0x00, &[0x82, 0, 0, 0, 0, 0, 0, 0, 88, 1]),
            ],
            expect_pct(88, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "finalmouse.mouse",
            ep("fm", 0x361D, 0x0100, Some(0xFF00), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x04, &[0x02, 0x95, 0x00]),
                step_write(0x04, &[0x02, 0xE0, 0x00]),
                step_read(0x05, &[0x05, 0x26, 0x03, 91, 0xA0, 0x0F]),
            ],
            expect_pct(91, ChargeState::Unknown, Some(4000)),
        ));
        let mut scy_req = vec![0_u8; 63];
        scy_req[0] = 0x01;
        scy_req[62] = 0x09;
        let mut scy_resp = vec![0_u8; 63];
        scy_resp[0] = 0x01;
        scy_resp[2] = 1; // frame[3] != 0
        scy_resp[3] = 0; // frame[4]
        scy_resp[4] = 0; // frame[5]
        scy_resp[6] = 4; // frame[7]
        scy_resp[19] = 69; // frame[20]
        fixtures.push(make_fixture(
            "scyrox.v12m",
            ep("scyrox", 0x395A, 0x0006, Some(0xFF00), Some(2), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x08, &scy_req),
                step_read(0x08, &scy_resp),
            ],
            expect_pct(69, ChargeState::Unknown, None),
        ));

        let json = br#"{"battery":77,"chr":1}"#;
        let mut rawm_stream = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let flen = (2 + json.len()) as u16;
        rawm_stream.push(0x02 | (((flen >> 8) as u8) << 4));
        rawm_stream.push((flen & 0xFF) as u8);
        rawm_stream.extend_from_slice(json);
        let mut rawm_pkt = vec![0xC0, 0x80 | (rawm_stream.len() as u8)];
        rawm_pkt.extend_from_slice(&rawm_stream);
        let rawm_req = padded(
            &[
                0xC0, 0x8D, 0x01, 0x0D, 0x03, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            64,
        );
        fixtures.push(make_fixture(
            "rawm.mouse",
            ep("rawm", 0x1915, 0x2346, Some(0xFF00), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &rawm_req),
                step_read(0x00, &rawm_pkt),
            ],
            expect_pct(77, ChargeState::Charging, None),
        ));

        fixtures.push(make_fixture(
            "zowie.ec_cw",
            ep("zowie", 0x04A5, 0x8006, Some(0xFF00), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0xA5, 0x03], 64)),
                step_read(0x00, &[0xA0, 0x0F, 75]),
            ],
            expect_pct(75, ChargeState::Unknown, Some(4000)),
        ));
        fixtures.push(make_fixture(
            "coolermaster.mouse",
            ep("cm", 0x2516, 0x01B7, Some(0xFF0A), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x46, 0x81], 31)),
                step_read(0x00, &[0x47, 0x81, 0x50, 0, 0, 0, 0, 0, 0, 0, 82, 0, 1]),
            ],
            expect_pct(82, ChargeState::Discharging, None),
        ));
        let mut nzxt_resp = vec![0_u8; 32];
        nzxt_resp[..3].copy_from_slice(&[0x4E, 0x02, 0x97]);
        nzxt_resp[8..10].copy_from_slice(&4000_u16.to_be_bytes());
        nzxt_resp[17] = 61;
        nzxt_resp[22] = 0x01;
        fixtures.push(make_fixture(
            "nzxt.lift_elite",
            ep("nzxt", 0x1E71, 0x2101, Some(0xFFCA), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x4E, 0x02, 0x81, 0x00, 0xB0], 64)),
                step_read(0x00, &nzxt_resp),
            ],
            expect_pct(61, ChargeState::Charging, Some(4000)),
        ));
        let mut msi_resp = vec![0_u8; 16];
        msi_resp[0] = 0x0F;
        msi_resp[1] = 0x20;
        msi_resp[14] = 84;
        fixtures.push(make_fixture(
            "msi.mouse",
            ep(
                "msi",
                0x0DB0,
                0x9E22,
                Some(0xFF0D),
                Some(0x00B0),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x0F, 0x20], 64)),
                step_read(0x00, &msi_resp),
            ],
            expect_pct(84, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "msi.gm41",
            ep("msi41", 0x0DB0, 0x0D4C, Some(0xFF10), Some(6), None, None),
            vec![step_read(0x00, &[0xA0, 100, 0x07, 0, 0, 0, 0])],
            expect_pct(69, ChargeState::Unknown, None),
        ));
        fixtures.push(make_fixture(
            "fantech.xd5",
            ep("fantech", 0x25A7, 0xFA7C, Some(0xFF02), Some(2), None, None),
            vec![
                step_send_feature(
                    0x08,
                    &padded(&[0x04, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x49], 16),
                ),
                step_read(0x08, &[0x04, 0, 0, 0, 0, 79, 0]),
            ],
            expect_pct(79, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "raigor.mouse",
            ep("raigor", 0x1EA7, 0x0066, Some(0xFFB5), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0xB5, &[0x05, 0, 0, 0, 0, 0, 0]),
                step_read(0xB5, &[0x05, 0x94, 0x04, 0, 0, 55, 0]),
            ],
            expect_pct(55, ChargeState::Unknown, None),
        ));
        fixtures.push(make_fixture(
            "sinowealth.receiver",
            ep("sino", 0x258A, 0x002F, Some(0xFF00), Some(1), None, None),
            vec![
                step_send_feature(0x05, &[0x80, 0, 0, 0, 0, 0, 0]),
                step_get_feature(0x05, 7, &[0x80, 0x01, 0, 0, 0, 0, 0]),
                step_send_feature(0x05, &[0x90, 0, 0, 0, 0, 0, 0]),
                step_get_feature(0x05, 7, &[0x90, 0x11, 73, 0, 0, 0, 0]),
            ],
            expect_pct(73, ChargeState::Unknown, None),
        ));
        fixtures.push(make_fixture(
            "royal_kludge.mg6_pro",
            ep("rk", 0x2E3C, 0x00A6, Some(0xFF00), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x01, 0x01, 0x02, 0x01], 64)),
                step_read(
                    0x00,
                    &[0x01, 0x01, 0x02, 0x10, 0xA0, 0x0F, 88, 0x00, 0, 0, 0],
                ),
            ],
            expect_pct(88, ChargeState::Discharging, Some(4000)),
        ));
        fixtures.push(make_fixture(
            "darmoshark.m3",
            ep("dm3", 0x248A, 0xFF30, Some(0x008C), Some(1), None, None),
            vec![
                step_send_feature(0x51, &padded(&[0x06], 20)),
                step_get_feature(
                    0x51,
                    20,
                    &padded(&[0x06, 0x01, 0x00, 0x8A, 0x24, 0x12, 0xFF, 0, 0, 0, 81], 20),
                ),
            ],
            expect_pct(81, ChargeState::Unknown, None),
        ));
        let mut dn_req = vec![0_u8; 64];
        dn_req[0] = 0x41;
        dn_req[2] = 0x81;
        dn_req[3] = 0x01;
        dn_req[63] = 0xA1_u8.wrapping_sub(sum8(&dn_req[..63]));
        let mut dn_resp = vec![0_u8; 64];
        dn_resp[0] = 0x41;
        dn_resp[2] = 0x8C;
        dn_resp[3] = 0x01;
        dn_resp[6] = 66;
        dn_resp[63] = 0xA1_u8.wrapping_sub(sum8(&dn_resp[..63]));
        fixtures.push(make_fixture(
            "darmoshark.nrf",
            ep("dnrf", 0x1915, 0x0723, Some(0xFF0A), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &dn_req),
                step_read(0x00, &dn_resp),
            ],
            expect_pct(66, ChargeState::Unknown, None),
        ));
        fixtures.push(make_fixture(
            "machenike.l7",
            ep("mach", 0x248A, 0xFF4C, Some(0x008C), Some(1), None, None),
            vec![
                step_send_feature(0x51, &padded(&[0x06], 20)),
                step_get_feature(
                    0x51,
                    20,
                    &padded(&[0x06, 0, 0, 0x8A, 0x24, 0x4E, 0xFF, 0, 0, 0x09, 70, 0], 20),
                ),
            ],
            expect_pct(70, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "niceboy.mw400",
            ep("niceboy", 0x248A, 0xFA02, Some(1), Some(0), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x05, &padded(&[0x24, 0xFA, 0x48], 32)),
                step_read(0x05, &[0x00, 0xFA, 0x48, 0xE8, 0x03, 0x01]), // 1000*990/240 = 4125mV -> 90%
            ],
            expect_pct(90, ChargeState::Charging, Some(4125)),
        ));
        let mut ms_resp = vec![0_u8; 21];
        ms_resp[19] = 128 + 65; // frame[20]
        fixtures.push(make_fixture(
            "motospeed.x6",
            ep("msx6", 0x0BDA, 0xFFE0, Some(0xFF00), Some(1), Some(4), None),
            vec![
                step_flush_timeout(),
                step_write(0xB5, &padded(&[0x06], 20)),
                step_read(0xB4, &ms_resp),
            ],
            expect_pct(65, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "xenta.mouse",
            ep("xenta", 0x1D57, 0xFA65, Some(0x000A), Some(1), None, None),
            vec![step_read(0x03, &[0x73, 0x40, 0x01, 83])],
            expect_pct(83, ChargeState::Discharging, None),
        ));
        let mut delux = vec![0_u8; 32];
        delux[1] = 0x20;
        delux[5] = 1; // frame[6]
        delux[17] = 74; // frame[18]
        delux[18] = 0; // frame[19]
        fixtures.push(make_fixture(
            "delux.m800_pro",
            ep("delux", 0x248A, 0x5B2F, Some(0xFF00), Some(1), None, None),
            vec![
                step_send_feature(0x0C, &padded(&[0x01, 0x20, 0x00, 0x01], 32)),
                step_get_feature(0x0C, 32, &delux),
            ],
            expect_pct(74, ChargeState::Discharging, None),
        ));
        let mut aj179 = vec![0_u8; 32];
        aj179[0] = 0x10;
        aj179[12] = 0;
        aj179[13] = 77;
        aj179[14] = 1;
        aj179[31] = sum8(&aj179[4..31]);
        fixtures.push(make_fixture(
            "ajazz.aj179",
            ep(
                "aj179",
                0x249A,
                0x0001,
                Some(0xFF00),
                Some(1),
                Some(2),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x10], 32)),
                step_read(0x00, &aj179),
            ],
            expect_pct(77, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "ajazz.aj_series",
            ep("aj159", 0x3151, 0x4099, Some(0xFFFF), Some(2), None, None),
            vec![
                step_send_feature(0x00, &padded(&[0xF7], 64)),
                step_get_feature(0x05, 64, &padded(&[0, 0, 86], 64)),
            ],
            expect_pct(86, ChargeState::Unknown, None),
        ));
        fixtures.push(make_fixture(
            "incott.mouse",
            ep("incott", 0x093A, 0x522C, Some(0xFF05), Some(1), None, None),
            vec![
                step_send_feature(0x09, &padded(&[0x89], 8)),
                step_get_feature(0x09, 8, &padded(&[0x89, 0x80 | 59, 0x01], 8)),
            ],
            expect_pct(59, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "rongyuan.receiver",
            ep(
                "rongyuan",
                0x3151,
                0x402D,
                Some(0xFFFF),
                Some(2),
                None,
                None,
            ),
            vec![
                step_send_feature(0x00, &padded(&[0xF7], 64)),
                step_get_feature(0x00, 64, &padded(&[0x00, 88, 64, 0, 0, 1], 64)),
            ],
            expect_pct(64, ChargeState::Unknown, None),
        ));
    }

    // --- Keyboards & Misc (Keychron, Cherry, Akko, NuPhy, Evision, AULA, SONiX, VIA, Semico, MCHOSE, VGN, DAREU, Rapoo, PressPlay, MAONO, DARKRIM) ---
    {
        let mut nape_slot = vec![0_u8; 32];
        nape_slot[0] = 0xB2;
        nape_slot[4] = 0x40;
        nape_slot[5] = 0x04;
        nape_slot[6] = 0x01;
        fixtures.push(make_fixture(
            "keychron.nape",
            ep(
                "kc_nape",
                0x3434,
                0xD026,
                Some(0xFF60),
                Some(0x0061),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0xB2], 32)),
                step_read(0x00, &nape_slot),
                step_write(0x00, &padded(&[0xA7, 0x31], 32)),
                step_read(0x00, &[0xA7, 0x31, 91, 1]),
            ],
            expect_pct(91, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "keychron.legacy_mouse",
            ep("kc_leg", 0x3434, 0xD030, Some(0x008C), Some(1), None, None),
            vec![
                step_send_feature(0x51, &padded(&[0x06], 20)),
                step_get_feature(
                    0x51,
                    20,
                    &padded(
                        &[0x06, 0x01, 0x00, 0x34, 0x34, 0x35, 0xD0, 0, 0, 0, 82, 0],
                        20,
                    ),
                ),
            ],
            expect_pct(82, ChargeState::Discharging, None),
        ));
        let mut ul_slot = vec![0_u8; 20];
        ul_slot[0] = 0x03;
        ul_slot[2] = 0x01;
        ul_slot[6] = 0x01;
        let mut ul_bat = vec![0_u8; 20];
        ul_bat[0] = 0x06;
        ul_bat[19] = 0x80 | 63;
        fixtures.push(make_fixture(
            "keychron.ultralink",
            ep("kc_ul", 0x3434, 0xD028, Some(0xFFC1), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0xB5, &padded(&[0x03], 20)),
                step_read(0xB5, &ul_slot),
                step_write(0xB3, &padded(&[0x06], 20)),
                step_read(0xB3, &ul_bat),
            ],
            expect_pct(63, ChargeState::Charging, None),
        ));
        let mut kcl_req = vec![0_u8; 64];
        kcl_req[0] = 0x01;
        kcl_req[2] = 0x81;
        kcl_req[3] = 0x01;
        kcl_req[63] = 0xA1_u8.wrapping_sub(sum8(&kcl_req[..63]));
        let mut kcl_resp = vec![0_u8; 64];
        kcl_resp[0] = 0x01;
        kcl_resp[3] = 0x01;
        kcl_resp[6] = 0x34;
        kcl_resp[7] = 0x34;
        kcl_resp[10] = 0;
        kcl_resp[11] = 75;
        fixtures.push(make_fixture(
            "keychron.launcher_mouse",
            ep("kc_l", 0x3434, 0xD038, Some(0xFF00), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &kcl_req),
                step_read(0x00, &kcl_resp),
            ],
            expect_pct(75, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "keychron.qmk_keyboard",
            ep(
                "kc_qmk",
                0x3434,
                0x0A50,
                Some(0xFF60),
                Some(0x0061),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0xA4], 32)),
                step_read(0x00, &[0xA4, 88, 0xA0, 0x0F, 0x00, 0, 0, 0]),
            ],
            expect_pct(88, ChargeState::Discharging, Some(4000)),
        ));

        for (id, vid, pid) in [
            ("cherry.mx3s", 0x046A_u16, 0x00EA_u16),
            ("akko.dash", 0x38EE, 0x000C),
        ] {
            fixtures.push(make_fixture(
                id,
                ep(id, vid, pid, Some(0xFF1C), Some(0x0092), None, None),
                vec![
                    step_flush_timeout(),
                    step_write(
                        0x04,
                        &padded(&[0x72, 0x00, 0x1A, 0x03, 0x00, 0x00, 0x55], 63),
                    ),
                    step_read(0x04, &[0x72, 0x00, 0x1A, 0, 0, 0, 0, 81, 1]),
                ],
                expect_pct(81, ChargeState::Charging, None),
            ));
        }

        let mut np_resp = vec![0_u8; 64];
        np_resp[..3].copy_from_slice(&[0xAA, 0x05, 0x00]);
        np_resp[4..8].copy_from_slice(&[0x01, 0x20, 0x00, 0x00]);
        np_resp[8] = 0x80 | 70;
        np_resp[3] = sum8(&np_resp[4..64]);
        fixtures.push(make_fixture(
            "nuphy.wh80",
            ep("nuphy", 0x19F5, 0xA011, Some(1), Some(0), None, None),
            vec![
                step_flush_timeout(),
                step_write(
                    0x00,
                    &padded(&[0x55, 0x05, 0x00, 0x21, 0x01, 0x20, 0x00, 0x00], 64),
                ),
                step_read(0x00, &np_resp),
            ],
            expect_pct(70, ChargeState::Charging, None),
        ));

        for (id, vid, pid, sub) in [
            ("evision.atk_keyboard", 0x320F_u16, 0x5055_u16, 0x00_u8),
            ("atk.z87", 0x373B, 0x1012, 0x00),
            ("evision.redragon_m999", 0x320F, 0x226F, 0x02),
            ("evision.ajazz_ak992", 0x320F, 0x50C3, 0x00),
        ] {
            fixtures.push(make_fixture(
                id,
                ep(id, vid, pid, Some(0xFF1C), Some(0x0092), None, None),
                vec![
                    step_flush_timeout(),
                    step_write(0x04, &padded(&[0x01, 0x00, 0x1A, sub], 63)),
                    step_read(0x04, &[0x01, 0x00, 0x1A, 0x00, 0x00, 0x00, 0x00, 79, 0]),
                ],
                expect_pct(79, ChargeState::Discharging, None),
            ));
        }

        fixtures.push(make_fixture(
            "evision.aula_mouse",
            ep(
                "aula_m",
                0x320F,
                0x22A0,
                Some(0xFF1C),
                Some(0x0092),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x04, &padded(&[0x00, 0x00, 0x01], 63)),
                step_read(0x04, &[0x00, 0x00, 0x01, 0, 0, 0, 0]),
                step_write(0x04, &padded(&[0x00, 0x00, 0x1A], 63)),
                step_read(0x04, &[0x00, 0x00, 0x1A, 0, 0, 0, 0, 88, 0]),
                step_write(0x04, &padded(&[0x00, 0x00, 0x02], 63)),
            ],
            expect_pct(88, ChargeState::Discharging, None),
        ));

        let mut h68_msg = [0_u8; 56];
        h68_msg[..5].copy_from_slice(&[0x09, 0x87, 0x00, 0x00, 0x01]);
        let mut h68_steps = vec![step_flush_timeout()];
        let mut c: u8 = 1;
        for idx in 1..=4_u8 {
            let mut f = [0_u8; 20];
            f[0] = 0x09;
            f[1] = 0x66;
            f[2] = 0x05 | if (c & 4) != 0 { 0x80 } else { 0 };
            f[3] = idx | if (c & 2) != 0 { 0x80 } else { 0 };
            f[4] = 0x0E | if (c & 1) != 0 { 0x80 } else { 0 };
            let off = usize::from(idx - 1) * 14;
            f[5..19].copy_from_slice(&h68_msg[off..off + 14]);
            f[19] = sum8(&f[1..19]);
            h68_steps.push(step_write(0x09, &f[1..]));
            c = c.wrapping_add(1) & 7;
        }
        let mut tr = [0_u8; 20];
        tr[0] = 0x09;
        tr[1] = 0x66;
        tr[2] = 0x05 | if (c & 4) != 0 { 0x80 } else { 0 };
        tr[3] = 5 | if (c & 2) != 0 { 0x80 } else { 0 };
        tr[4] = 0x08 | if (c & 1) != 0 { 0x80 } else { 0 };
        tr[12] = 0xFF_u8.wrapping_sub(sum8(&h68_msg));
        tr[13] = sum8(&tr[1..13]);
        h68_steps.push(step_write(0x09, &tr[1..]));
        let mut h68_resp = [0_u8; 20];
        h68_resp[0] = 0x09;
        h68_resp[1] = 0x66;
        h68_resp[3] = 1;
        h68_resp[5] = 0x87;
        h68_resp[11] = 67;
        h68_resp[12] = 0x10;
        h68_resp[19] = sum8(&h68_resp[1..19]);
        h68_steps.push(step_read(0x09, &h68_resp[1..]));
        fixtures.push(make_fixture(
            "aula.hero68",
            ep(
                "hero68",
                0x372E,
                0x106C,
                Some(0xFF60),
                Some(0x0061),
                None,
                None,
            ),
            h68_steps,
            expect_pct(67, ChargeState::Charging, None),
        ));

        let mut sc580 = vec![0_u8; 32];
        sc580[0] = 0x30;
        sc580[1] = 0x00;
        sc580[12] = 0; // discharging
        sc580[13] = 73;
        sc580[14] = 1; // awake
        sc580[31] = sum8(&sc580[4..31]);
        fixtures.push(make_fixture(
            "aula.sc580",
            ep("sc580", 0x089D, 0x062F, Some(0xFF02), Some(0), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x30, 0x00, 0x01], 32)),
                step_read(0x00, &sc580),
            ],
            expect_pct(73, ChargeState::Discharging, None),
        ));

        let mut sonix_req = vec![0_u8; 32];
        sonix_req[0] = 0x20;
        sonix_req[1] = 0x01;
        sonix_req[31] = 0x21;
        fixtures.push(make_fixture(
            "sonix.keyboard",
            ep(
                "sonix",
                0x05AC,
                0x024F,
                Some(0xFF60),
                Some(0x0061),
                Some(3),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &sonix_req),
                step_read(0x00, &[0x20, 0x01, 0x00, 84]),
            ],
            expect_pct(84, ChargeState::Unknown, None),
        ));

        fixtures.push(make_fixture(
            "via.dongle",
            ep(
                "via",
                0x36B0,
                0x3002,
                Some(0xFF60),
                Some(0x0061),
                None,
                Some("URX Core75"),
            ),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0x08, 0x00, 0x07], 32)),
                step_read(0x00, &[0x08, 0x00, 0x07, 92]),
            ],
            expect_pct(92, ChargeState::Unknown, None),
        ));

        fixtures.push(make_fixture(
            "semico.keyboard",
            ep("semico", 0x1A2C, 0x8FFF, Some(0xFF02), Some(2), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &padded(&[0xAA, 0x55, 0xCC, 0x33, 0x0E], 64)),
                step_read(0x00, &[0xAA, 0x55, 0xCC, 0x33, 0x0E, 71, 0x01]),
            ],
            expect_pct(71, ChargeState::Unknown, None),
        ));

        // MCHOSE new variants
        fixtures.push(make_fixture(
            "mchose.g3_v2",
            ep(
                "mc_g3",
                0x3837,
                0x4245,
                Some(0xFF01),
                Some(0x0010),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(
                    0x00,
                    &padded(&[0x55, 0x30, 0xA5, 0x0B, 0x2E, 0x01, 0x01, 0x01], 64),
                ),
                step_read(0x00, &[0xAA, 0x30, 0, 0, 0, 0, 0, 0, 85, 0]),
            ],
            expect_pct(85, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "mchose.v9_pro2",
            ep(
                "mc_v9p2",
                0x3837,
                0x6011,
                Some(0xFF07),
                Some(0x0222),
                None,
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(
                    0x55,
                    &padded(&[0xAA, 0x01, 0x00, 0x01, 0x00, 0x01, 0x0B], 63),
                ),
                step_read(
                    0x55,
                    &[
                        0xAA,
                        0x01,
                        0x00,
                        0x03,
                        0x00,
                        0x01,
                        0x0B,
                        0x01,
                        0x80 | 64,
                        0x00,
                    ],
                ),
            ],
            expect_pct(64, ChargeState::Charging, None),
        ));
        let mut glw_resp = vec![0_u8; 64];
        glw_resp[0] = 0xAA;
        glw_resp[1] = 0x05;
        glw_resp[4] = 33;
        glw_resp[40] = 88;
        glw_resp[3] = sum8(&glw_resp[4..41]);
        fixtures.push(make_fixture(
            "mchose.glw_keyboard",
            ep("mc_glw", 0x3837, 0x3033, Some(1), Some(0), None, None),
            vec![
                step_flush_timeout(),
                step_write(
                    0x00,
                    &padded(&[0x55, 0x05, 0x00, 0x38, 0x38, 0x00, 0x00, 0x00], 64),
                ),
                step_read(0x00, &glw_resp),
            ],
            expect_pct(88, ChargeState::Discharging, None),
        ));
        let mc_mod_resp = |is_kb: bool| {
            let mut r = [0_u8; 64];
            r[0] = 0x4D;
            r[1] = 0x01;
            r[2] = 0x01;
            r[3] = 20;
            r[4] = 0x00;
            r[5] = 0x09;
            r[8] = 0x37;
            r[9] = 0x38;
            if is_kb {
                r[10] = 0x23;
                r[11] = 0x20;
            }
            r[18] = 0x02;
            r[19] = 0x01;
            r[20] = 75;
            r[28] = r[2..28].iter().fold(0_u8, |a, b| a ^ b);
            r[1..].to_vec()
        };
        fixtures.push(make_fixture(
            "mchose.modern_mouse",
            ep("mc_mm", 0x3837, 0x1018, Some(0xFF01), Some(1), None, None),
            vec![
                step_write(
                    0x4D,
                    &padded(&[0x01, 0x01, 0x00, 0x00, 0x09, 0x00, 0x00, 0x08], 63),
                ),
                step_read(0x4D, &mc_mod_resp(false)),
            ],
            expect_pct(75, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "mchose.k87_v3",
            ep("mc_k87", 0x3837, 0x1015, Some(0xFF80), Some(1), None, None),
            vec![
                step_write(
                    0x4D,
                    &padded(&[0x01, 0x01, 0x00, 0x00, 0x09, 0x00, 0x00, 0x08], 63),
                ),
                step_read(0x4D, &mc_mod_resp(true)),
            ],
            expect_pct(75, ChargeState::Charging, None),
        ));

        // VGN OMNISOUND, DAREU Togran/A84, Rapoo VT/Legacy, Press Play, MAONO, DARKRIM
        fixtures.push(make_fixture(
            "vgn.omnisound",
            ep("vgn", 0x372E, 0x10BB, Some(0x000C), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_set_output(0x55, &padded(&[0x65, 0x01], 63)),
                step_read(0x55, &[0x65, 83]),
            ],
            expect_pct(83, ChargeState::Unknown, None),
        ));
        let mut tog = vec![0_u8; 64];
        tog[0] = 0x02;
        tog[2] = 0x07;
        tog[3] = 0x80;
        tog[6] = 1; // charging
        tog[7] = 75;
        tog[8] = 100;
        fixtures.push(make_fixture(
            "dareu.togran",
            ep("dareu_t", 0x260D, 0x1148, Some(0x000C), Some(1), None, None),
            vec![
                step_send_feature(0x0A, &padded(&[0x00, 0x04, 0x07, 0x80], 64)),
                step_get_feature(0x0A, 64, &tog),
            ],
            expect_pct(75, ChargeState::Charging, None),
        ));
        let mut a84 = vec![0_u8; 64];
        a84[4] = 89; // frame[5]
        a84[14] = 1; // frame[15]
        fixtures.push(make_fixture(
            "dareu.a84",
            ep("dareu_a84", 0x260D, 0x0022, Some(1), Some(6), None, None),
            vec![
                step_send_feature(0x00, &padded(&[0x04, 0xF6], 64)),
                step_get_feature(0x00, 64, &a84),
            ],
            expect_pct(89, ChargeState::Unknown, None),
        ));
        fixtures.push(make_fixture(
            "rapoo.vt",
            ep(
                "rapoo_vt",
                0x24AE,
                0x1417,
                Some(0xFF00),
                Some(2),
                None,
                None,
            ),
            vec![step_read(0x07, &[0x00, 0, 0, 0, 0, 0, 0x02, 78])],
            expect_pct(78, ChargeState::Charging, None),
        ));
        fixtures.push(make_fixture(
            "rapoo.legacy_vt3_pro",
            ep(
                "rapoo_leg",
                0x24AE,
                0x1215,
                Some(0xFF00),
                Some(2),
                None,
                None,
            ),
            vec![step_read(0xBB, &[0xB0, 0, 0, 0, 0x01, 62])],
            expect_pct(62, ChargeState::Discharging, None),
        ));
        fixtures.push(make_fixture(
            "pressplay.icarus",
            ep("icarus", 0x7768, 0x6434, Some(0xFFB2), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_set_output(0x06, &padded(&[0x05], 63)),
                step_read(0x06, &[0x05, 0x26, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]),
            ],
            expect_pct(90, ChargeState::Unknown, None),
        ));
        let mut maono_resp = vec![0x0B, 0x00, 0x00, 0x04, 0x42, 0x00, 82, 0x00, 0x00, 0x00];
        let mut full_m = vec![0xC4];
        full_m.extend_from_slice(&maono_resp[..8]);
        let msum: u32 = full_m.iter().map(|&b| u32::from(b)).sum();
        let mchk = ((0x10000_u32.wrapping_sub(msum & 0xFFFF)) & 0xFFFF) as u16;
        maono_resp[8] = (mchk & 0xFF) as u8;
        maono_resp[9] = (mchk >> 8) as u8;
        fixtures.push(make_fixture(
            "maono.pd200w",
            ep(
                "maono",
                0x352F,
                0x040E,
                Some(0x000C),
                Some(1),
                Some(3),
                None,
            ),
            vec![
                step_flush_timeout(),
                step_write(
                    0xC4,
                    &padded(&[0x09, 0x00, 0x00, 0x04, 0x42, 0x00, 0xED, 0xFE], 63),
                ),
                step_read(0xC4, &maono_resp),
            ],
            expect_pct(82, ChargeState::Unknown, None),
        ));

        let dk_body = [
            0xFE, 0x01, 0x10, 0x00, 0x01, 0x00, 0x01, 0x02, 0x00, 0x01, 0x00,
        ];
        let dk_crc = crc16(&dk_body, 0xFFFF);
        let mut dk_req = vec![0_u8; 64];
        dk_req[..11].copy_from_slice(&dk_body);
        dk_req[11] = (dk_crc >> 8) as u8;
        dk_req[12] = (dk_crc & 0xFF) as u8;
        let mut dk_resp = vec![
            0xFE,
            0x01,
            0x10,
            0x00,
            0x02,
            0x00,
            0x00,
            0x03,
            0x00,
            0x01,
            0x01,
            0x80 | 70,
        ];
        let rcrc = crc16(&dk_resp, 0xFFFF);
        dk_resp.push((rcrc >> 8) as u8);
        dk_resp.push((rcrc & 0xFF) as u8);
        fixtures.push(make_fixture(
            "darkrim.shadow",
            ep("darkrim", 0x3654, 0x6901, Some(0xFF02), Some(2), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x00, &dk_req),
                step_read(0x00, &dk_resp),
            ],
            expect_pct(70, ChargeState::Charging, None),
        ));
    }

    let mut tested_ids = HashSet::new();
    for fixture in &fixtures {
        verify_fixture(fixture, registry.clone())
            .unwrap_or_else(|err| panic!("fixture {} failed: {err}", fixture.case));
        tested_ids.insert(fixture.target.id.clone());
    }

    // Ensure every single provider in Registry::builtin() is accounted for (either existing JSON
    // fixtures in fixtures/vendors/ or this extended suite, plus bluetooth.bas).
    let existing_json_ids: HashSet<&str> = [
        "asus.headset",
        "asus.rog_omni",
        "atk.a9_air",
        "atk.n9v2",
        "bluetooth.bas",
        "crociris.x7",
        "dareu.a950_air_v2",
        "eweadn.a5_pro",
        "eweadn.s9_pro",
        "imecoo.p1",
        "logitech.hidpp",
        "mchose.a5_mouse",
        "mchose.a7_ultra",
        "mchose.k7_v2_ultra",
        "mchose.keyboard",
        "mchose.v9",
        "os.xinput",
        "rapoo.vt3s",
        "razer.headset",
        "razer.mouse",
        "sony.dualsense",
        "taidu.mouse",
        "vxe.r1",
        "wob.rainy75",
        "xiberia.t7",
    ]
    .into_iter()
    .collect();

    let missing: Vec<&str> = registry
        .providers()
        .iter()
        .map(|p| p.id())
        .filter(|id| !tested_ids.contains(*id) && !existing_json_ids.contains(*id))
        .collect();
    assert!(
        missing.is_empty(),
        "every builtin provider must have a strict replay test; missing: {missing:?}"
    );
    assert!(
        registry.providers().len() >= 110,
        "expected >= 110 builtin providers, got {}",
        registry.providers().len()
    );
}
