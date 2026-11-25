//! Replay tests for Sony, Gamepads, SteelSeries, Corsair, HyperX, Logitech audio, and headsets.

use super::common::*;
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::{Expectation, Fixture};
use wireless_battery::{BatteryLevel, ChargeState, Precision, Reachability};

pub(super) fn sony_and_gamepad_fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();

    // --- Sony DualShock 4 & INZONE ---
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
    resp[0] = 14;
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

    fixtures.push(make_fixture(
        "sony.pulse_3d",
        ep(
            "pulse3d",
            0x054C,
            0x0D5E,
            Some(0xFF01),
            Some(0x0020),
            None,
            None,
        ),
        vec![
            step_read(0xB0, &[0x00, 0x00, 78, 0x08, 0x00, 0x00, 0x00]),
            step_flush_timeout(),
        ],
        expect_pct(78, ChargeState::Discharging, None),
    ));

    // --- Gamepads ---
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

    let mut cyc = vec![0_u8; 63];
    cyc[34] = 0x01;
    cyc[35] = 64;
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

    let mut sj = vec![0_u8; 63];
    sj[0] = 14;
    sj[1] = 0x0A;
    sj[2] = 0x01;
    sj[12] = 77;
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

    let mut fd = vec![0_u8; 32];
    fd[..3].copy_from_slice(&[0x5A, 0xA5, 0x01]);
    fd[5] = 0x82;
    fd[11] = 0x14;
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

    let mut eb_link = vec![0_u8; 63];
    eb_link[0] = 0x04;
    eb_link[3] = 0x20;
    eb_link[4] = 0x01;
    eb_link[17] = 0x01;
    let mut eb_bat = vec![0_u8; 63];
    eb_bat[0] = 0x04;
    eb_bat[3] = 0x21;
    eb_bat[4] = 0x01;
    eb_bat[17] = 0x80 | 66;
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

    let mut ebd = vec![0_u8; 34];
    ebd[13] = 92;
    fixtures.push(make_fixture(
        "eightbitdo.dinput",
        ep("8bitdo_d", 0x2DC8, 0x6012, Some(1), Some(5), None, None),
        vec![step_read(0x01, &ebd)],
        expect_pct(92, ChargeState::Discharging, None),
    ));

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

    fixtures
}

pub(super) fn steelseries_and_corsair_fixtures() -> Vec<Fixture> {
    let mut fixtures = vec![
        make_fixture(
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
        ),
        make_fixture(
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
        ),
        make_fixture(
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
        ),
        make_fixture(
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
        ),
    ];
    let mut npw = vec![0_u8; 63];
    npw[0] = 0xB0;
    npw[5] = 6;
    npw[14] = 0x02;
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
    ne[5] = 91;
    ne[13] = 0x03;
    ne[14] = 0x00;
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
            step_read(0x00, &[0xD2, 17]),
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

    // --- Corsair ---
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
    br_bat[0] = 0x01;
    br_bat[1] = 0x02;
    br_bat[2] = 0x00;
    br_bat[3..7].copy_from_slice(&840_u32.to_le_bytes());
    let mut br_chg = vec![0_u8; 63];
    br_chg[0] = 0x01;
    br_chg[1] = 0x02;
    br_chg[2] = 0x00;
    br_chg[3..7].copy_from_slice(&1_u32.to_le_bytes());
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

    fixtures
}

pub(super) fn hyperx_and_logitech_audio_fixtures() -> Vec<Fixture> {
    let mut fixtures = vec![
        make_fixture(
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
                step_read(0x66, &[0x89, 0x0F, 0xA0, 90]),
            ],
            expect_pct(90, ChargeState::Discharging, Some(4000)),
        ),
        make_fixture(
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
        ),
        make_fixture(
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
        ),
        make_fixture(
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
        ),
        make_fixture(
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
        ),
        make_fixture(
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
        ),
        make_fixture(
            "hyperx.cloud_flight",
            ep("hx_cf", 0x0951, 0x16C4, Some(0xFF90), Some(1), None, None),
            vec![
                step_flush_timeout(),
                step_write(0x21, &padded(&[0xFF, 0x05], 19)),
                step_read(0x21, &[0xFF, 0x05, 0x0F, 0xA0]),
            ],
            expect_pct(100, ChargeState::Discharging, Some(4000)),
        ),
    ];
    let mut sc_conn = vec![0_u8; 31];
    sc_conn[10] = 0x01;
    let mut sc_bat = vec![0_u8; 31];
    sc_bat[0] = 0x12;
    sc_bat[1] = 0x05;
    sc_bat[8] = 0x03;
    sc_bat[10..12].copy_from_slice(&4042_u16.to_le_bytes());
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

    // --- Logitech G435 & ASTRO ---
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
    g435_resp[10..18].copy_from_slice(&[0x00, 0x07, 0x00, 0x06, 0x05, 0x00, 0xA0, 0x0F]);
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
            step_read(0x02, &[0x02, 0x01, 0x02]),
            step_flush_timeout(),
            step_write(0x02, &padded(&[0x7C], 63)),
            step_read(0x02, &[0x02, 0x01, 69]),
        ],
        expect_pct(69, ChargeState::Discharging, None),
    ));

    // --- Logitech Centurion, G930, and HID++ 1.0 ---
    fixtures.push(make_fixture(
        "logitech.centurion",
        ep(
            "logi_cent",
            0x046D,
            0x0AF7,
            Some(0xFFA0),
            Some(0x0001),
            Some(3),
            None,
        ),
        vec![
            step_write(0x51, &padded(&[5, 0x00, 0x00, 0x01, 0x00, 0x03], 63)),
            step_read(0x51, &padded(&[4, 0x00, 0x00, 0x01, 0x03], 63)),
            step_write(
                0x51,
                &padded(
                    &[
                        10, 0x00, 0x03, 0x11, 0x00, 0x05, 0x00, 0x00, 0x01, 0x01, 0x04,
                    ],
                    63,
                ),
            ),
            step_read(0x51, &padded(&[3, 0x00, 0x03, 0x11], 63)),
            step_read(
                0x51,
                &padded(
                    &[9, 0x00, 0x03, 0x10, 0x00, 0x04, 0x00, 0x00, 0x01, 0x04],
                    63,
                ),
            ),
            step_write(
                0x51,
                &padded(&[8, 0x00, 0x03, 0x11, 0x00, 0x03, 0x00, 0x04, 0x01], 63),
            ),
            step_read(0x51, &padded(&[3, 0x00, 0x03, 0x11], 63)),
            step_read(
                0x51,
                &padded(
                    &[
                        11, 0x00, 0x03, 0x10, 0x00, 0x06, 0x00, 0x04, 0x01, 82, 0x00, 0x01,
                    ],
                    63,
                ),
            ),
        ],
        expect_pct(82, ChargeState::Charging, None),
    ));

    let mut g930_resp = vec![0_u8; 64];
    g930_resp[12] = 91; // raw 91 -> 100%
    fixtures.push(make_fixture(
        "logitech.g930",
        ep(
            "g930",
            0x046D,
            0x0A1F,
            Some(0xFF00),
            Some(0x0001),
            None,
            None,
        ),
        vec![
            step_send_feature(
                0xFF,
                &padded(
                    &[0x09, 0x00, 0xFD, 0xF4, 0x10, 0x05, 0xB1, 0xBF, 0xA0, 0x04],
                    63,
                ),
            ),
            step_get_feature(0xFF, 64, &g930_resp),
            step_get_feature(0xFF, 64, &g930_resp),
            step_get_feature(0xFF, 64, &g930_resp),
        ],
        expect_pct(100, ChargeState::Unknown, None),
    ));

    fixtures.push(make_fixture(
        "logitech.hidpp10",
        ep(
            "hidpp10",
            0x046D,
            0xC52F,
            Some(0xFF00),
            Some(0x0001),
            None,
            Some("Logitech Mouse"),
        ),
        vec![
            step_write(0x10, &[0x00, 0x81, 0x0D, 0x00, 0x00, 0x00]),
            step_read(0x10, &[0x00, 0x81, 0x0D, 75, 0x00, 0x40]),
        ],
        expect_pct(75, ChargeState::Charging, None),
    ));

    fixtures
}

pub(super) fn headsets_audio_fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();
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
    akg_resp[0] = 9;
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
    tb_resp[0] = gsi.len() as u8;
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
                    0x01, 0x01, 0x10, 0x06, 0x02, 0x00, 0x00, 0x03, 0x0A, 0x1A, 0x04, 0x06, 0x01,
                ],
            ),
        ],
        expect_pct(80, ChargeState::Charging, None),
    ));

    let mut fd_dongle = vec![0_u8; 23];
    fd_dongle[0] = 0x11;
    fd_dongle[1] = 0x21;
    fd_dongle[3] = 1;
    let mut fd_hs = vec![0_u8; 23];
    fd_hs[0] = 0xF1;
    fd_hs[1] = 0x21;
    fd_hs[14] = 89;
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
    aw[2] = 0x09;
    aw[5] = 73;
    aw[6] = 1;
    aw[12] = aw[2..=11].iter().fold(0_u8, |a, b| a ^ b);
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

    fixtures.push(make_fixture(
        "lenovo.wireless_voip",
        ep(
            "lenovo_voip",
            0x17EF,
            0xA07D,
            Some(0xFF07),
            Some(0x0222),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_send_feature(0x24, &padded(&[0x01], 60)),
            step_read(0x27, &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 85]),
        ],
        expect_pct(85, ChargeState::Unknown, None),
    ));

    let crc8_epos = |data: &[u8]| -> u8 {
        let mut crc = 0xFF_u8;
        for &byte in data {
            crc ^= byte;
            for _ in 0..8 {
                if (crc & 0x80) != 0 {
                    crc = (crc << 1) ^ 0x07;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    };
    let epos_req_payload = |target: u16, group: u16, cmd: u16| {
        let mut payload = [0_u8; 4];
        payload[0..2].copy_from_slice(&group.to_le_bytes());
        payload[2..4].copy_from_slice(&cmd.to_le_bytes());
        let mut req = vec![0_u8; 106];
        req[0] = 0xEB;
        req[1] = 0xFE;
        req[2..4].copy_from_slice(&0x0001_u16.to_le_bytes());
        req[4..6].copy_from_slice(&0x0004_u16.to_le_bytes());
        req[6..8].copy_from_slice(&target.to_le_bytes());
        req[8..10].copy_from_slice(&(payload.len() as u16).to_le_bytes());
        req[10..12].copy_from_slice(&u16::from(crc8_epos(&payload)).to_le_bytes());
        req[12..16].copy_from_slice(&payload);
        req
    };
    let epos_resp_payload = |target: u16, group: u16, cmd: u16, val: u16| {
        let mut payload = [0_u8; 6];
        payload[0..2].copy_from_slice(&group.to_le_bytes());
        payload[2..4].copy_from_slice(&cmd.to_le_bytes());
        payload[4..6].copy_from_slice(&val.to_le_bytes());
        let mut resp = vec![0_u8; 18];
        resp[0] = 0xEB;
        resp[1] = 0xFE;
        resp[2..4].copy_from_slice(&0x0001_u16.to_le_bytes());
        resp[4..6].copy_from_slice(&0x0004_u16.to_le_bytes());
        resp[6..8].copy_from_slice(&target.to_le_bytes());
        resp[8..10].copy_from_slice(&(payload.len() as u16).to_le_bytes());
        resp[10..12].copy_from_slice(&u16::from(crc8_epos(&payload)).to_le_bytes());
        resp[12..18].copy_from_slice(&payload);
        resp
    };
    fixtures.push(make_fixture(
        "epos.h3pro_hybrid",
        ep(
            "epos_h3pro",
            0x1395,
            0x0303,
            Some(0xFFFF),
            Some(0x0001),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0x01, &epos_req_payload(0x0001, 0x0001, 0x0009)),
            step_read(0x01, &epos_resp_payload(0x0000, 0x0001, 0x0009, 1)),
            step_write(0x01, &epos_req_payload(0x0005, 0x0012, 0x0003)),
            step_read(0x01, &epos_resp_payload(0x0004, 0x0012, 0x0003, 76)),
        ],
        expect_pct(76, ChargeState::Unknown, None),
    ));

    fixtures.push(make_fixture(
        "fifine.ht6",
        ep(
            "fifine_ht6",
            0x3142,
            0x00C3,
            Some(0xFF90),
            Some(0x0001),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0x55, &padded(&[0x65, 0x01], 63)),
            step_read(0x55, &[0x65, 72, 0x03]),
        ],
        expect_pct(72, ChargeState::Charging, None),
    ));

    fixtures.push(make_fixture(
        "mchose.x9_v9pro",
        ep(
            "mchose_x9",
            0x3837,
            0x6045,
            Some(0xFF90),
            Some(0x0001),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0x55, &padded(&[0x65, 0x01], 63)),
            step_read(0x55, &[0x65, 91, 0x02]),
        ],
        expect_pct(91, ChargeState::Unknown, None),
    ));

    fixtures
}

#[test]
fn replay_sony_and_gamepads() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &sony_and_gamepad_fixtures());
}

#[test]
fn replay_steelseries_and_corsair() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &steelseries_and_corsair_fixtures());
}

#[test]
fn replay_hyperx_and_logitech_audio() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &hyperx_and_logitech_audio_fixtures());
}

#[test]
fn replay_headsets_audio() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &headsets_audio_fixtures());
}
