//! Negative, abnormal-path, tightened-matcher, and Family-vs-Exact priority tests.

use super::common::*;
use wireless_battery::providers::MatchDecision;
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::{verify_fixture, Expectation};
use wireless_battery::{BatteryLevel, ChargeState, DeviceKind, ErrorKind, Precision, Reachability};

#[test]
fn negative_out_of_range_percentage_bytes_are_rejected_not_clamped() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // corsair.void: 0x80 | 105 = 233 -> (pct = 105) must fail rather than clamping to 100%.
    let void_bad = make_fixture(
        "corsair.void",
        ep(
            "cor_void_bad",
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
            step_read(0xCA, &[0x00, 0x80 | 105, 0x00, 0x01]),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    // steelseries.arctis1: pct 101 must return InvalidResponse.
    let arctis1_bad = make_fixture(
        "steelseries.arctis1",
        ep(
            "ss_a1_bad",
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
            step_read(0x06, &[0x12, 0x03, 101]),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    // mchose.a5_gen1: pct 150 must return InvalidResponse instead of clamping 150 -> 100.
    let dc_req = padded(&[0x00, 0x00, 0x02, 0x02, 0x00, 0x83], 64);
    let a5_bad_resp = padded(&[0xA1, 0x00, 0x02, 0x02, 0x00, 0x83, 0x00, 150], 64);
    let mchose_a5_bad = make_fixture(
        "mchose.a5_gen1",
        ep(
            "mc_a5g1_bad",
            0x2023,
            0xF013,
            Some(0xFFFF),
            Some(0),
            None,
            None,
        ),
        vec![
            step_send_feature(0x00, &dc_req),
            step_get_feature(0x00, 64, &a5_bad_resp),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    // fantech.xd5 (0xFA7E scales by 10): raw byte 15 (150%) must return InvalidResponse.
    let xd5_bad = make_fixture(
        "fantech.xd5",
        ep(
            "fantech_bad",
            0x25A7,
            0xFA7E,
            Some(0xFF02),
            Some(2),
            None,
            None,
        ),
        vec![
            step_send_feature(
                0x08,
                &padded(&[0x04, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x49], 16),
            ),
            step_read(0x08, &[0x04, 0, 0, 0, 0, 15, 0]),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    verify_all(&registry, &[void_bad, arctis1_bad, mchose_a5_bad, xd5_bad]);
}

#[test]
fn negative_tightened_matchers_ignore_audio_consumer_interfaces() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    let non_vendor_endpoints = [
        ep(
            "ss_a9_audio",
            0x1038,
            0x12C2,
            Some(0x000C),
            Some(1),
            Some(0),
            None,
        ),
        ep(
            "ss_apw_audio",
            0x1038,
            0x1290,
            Some(0x0001),
            Some(6),
            Some(0),
            None,
        ),
        ep(
            "ss_npw_audio",
            0x1038,
            0x225D,
            Some(0x000C),
            Some(1),
            Some(4),
            None,
        ),
        ep(
            "rz_v3x_audio",
            0x1532,
            0x057D,
            Some(0x000C),
            Some(1),
            Some(0),
            None,
        ),
        ep(
            "rz_v3_audio",
            0x1532,
            0x0577,
            Some(0x000B),
            Some(5),
            Some(5),
            None,
        ),
        ep(
            "ms_x6_audio",
            0x0BDA,
            0xFFE0,
            Some(0x0001),
            Some(2),
            Some(4),
            None,
        ),
        ep(
            "aj_179_mouse",
            0x249A,
            0x0001,
            Some(0x0001),
            Some(2),
            Some(2),
            None,
        ),
        ep(
            "jbl_610_audio",
            0x0ECB,
            0x205C,
            Some(0x000C),
            Some(1),
            Some(5),
            None,
        ),
    ];

    for endpoint in &non_vendor_endpoints {
        let matches = match_providers(&registry, endpoint);
        assert!(
            matches.is_empty(),
            "non-vendor collection {:?} unexpectedly matched {:?}",
            endpoint.id,
            matches.iter().map(|(p, _)| p.id()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn negative_corrupted_headers_checksums_and_offline_behave_conservatively() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // Bad XOR checksum in mchose.k7_v2_ultra -> ChecksumFailed.
    let mut k7_bad = [0_u8; 64];
    k7_bad[..4].copy_from_slice(&[0x4D, 0x01, 0x01, 0x10]);
    k7_bad[5] = 0x09;
    k7_bad[8..12].copy_from_slice(&[0x37, 0x38, 0x26, 0x40]);
    k7_bad[18] = 0x02;
    k7_bad[20] = 80;
    k7_bad[24] = 0xFF; // corrupted checksum
    let k7_bad_fixture = make_fixture(
        "mchose.k7_v2_ultra",
        ep(
            "mc_k7_bad",
            0x3837,
            0x1014,
            Some(0xFF01),
            Some(1),
            Some(2),
            None,
        ),
        vec![
            step_write(
                0x4D,
                &padded(&[0x01, 0x01, 0x00, 0x00, 0x09, 0x00, 0x00, 0x08], 63),
            ),
            step_read(0x4D, &k7_bad[1..]),
        ],
        Expectation::Error {
            error: ErrorKind::ChecksumFailed,
        },
    );
    verify_fixture(&k7_bad_fixture, registry.clone())
        .expect("corrupted K7 checksum must fail cleanly");

    // hyperx.cloud3 offline report returns Offline with percentage: None (never 0%).
    let hx_offline = make_fixture(
        "hyperx.cloud3",
        ep(
            "hx_c3_off",
            0x03F0,
            0x05B7,
            Some(0xFF90),
            Some(1),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0x66, &padded(&[0x82], 61)),
            step_read(0x66, &[0x82, 0x00, 0x00, 0x00]),
        ],
        Expectation::Reading {
            percentage: None,
            level: None,
            voltage_mv: None,
            charge_state: ChargeState::Unknown,
            reachability: Reachability::Offline,
            precision: Precision::None,
        },
    );
    verify_fixture(&hx_offline, registry.clone())
        .expect("offline HyperX report must stay Offline with percentage None");

    // Tightened SteelSeries Nova Pro Wireless frame check: missing 0xB0 command byte triggers retry -> timeout.
    let mut npw_wrong_hdr = vec![0_u8; 63];
    npw_wrong_hdr[0] = 0x00; // missing 0xB0 command echo
    npw_wrong_hdr[5] = 6;
    let npw_timeout = make_fixture(
        "steelseries.nova_pro_wireless",
        ep(
            "ss_npw_hdr",
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
            step_read(0x06, &npw_wrong_hdr),
            step_flush_timeout(),
        ],
        Expectation::Error {
            error: ErrorKind::Timeout,
        },
    );
    verify_fixture(&npw_timeout, registry.clone())
        .expect("mismatched SteelSeries Nova Pro Wireless command header must be ignored");

    // Tightened HyperX Cloud Flight frame check: arbitrary 5-byte frame without 0x21 0xFF 0x05 is ignored -> timeout.
    let flight_timeout = make_fixture(
        "hyperx.cloud_flight",
        ep(
            "hx_cf_hdr",
            0x0951,
            0x16C4,
            Some(0xFF90),
            Some(1),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0x21, &padded(&[0xFF, 0x05], 19)),
            step_read(0x21, &[0xFF, 0x06, 0x0F, 0xA0]),
            step_flush_timeout(),
        ],
        Expectation::Error {
            error: ErrorKind::Timeout,
        },
    );
    verify_fixture(&flight_timeout, registry)
        .expect("mismatched HyperX Cloud Flight prefix must be ignored");
}

#[test]
fn family_matchers_yield_to_exact_providers_and_assign_device_kinds() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // Exact outranks Family on shared vendor/usage_page pairs.
    let cases = [
        (
            ep("vxe", 0x373B, 0x10C9, Some(0xFF02), Some(2), None, None),
            "vxe.r1",
            DeviceKind::Mouse,
        ),
        (
            ep(
                "mchose_k7",
                0x3837,
                0x1014,
                Some(0xFF01),
                Some(1),
                Some(2),
                None,
            ),
            "mchose.k7_v2_ultra",
            DeviceKind::Mouse,
        ),
        (
            ep(
                "rongyuan_3151",
                0x3151,
                0x402D,
                Some(0xFFFF),
                Some(2),
                None,
                None,
            ),
            "rongyuan.receiver",
            DeviceKind::Peripheral,
        ),
        (
            ep(
                "kc_nape",
                0x3434,
                0xD026,
                Some(0xFF60),
                Some(0x0061),
                None,
                None,
            ),
            "keychron.nape",
            DeviceKind::Mouse,
        ),
        (
            ep(
                "logi_mx_kb",
                0x046D,
                0x408A,
                Some(0xFF00),
                Some(2),
                None,
                Some("MX Keys Mini"),
            ),
            "logitech.hidpp",
            DeviceKind::Keyboard,
        ),
        (
            ep(
                "razer_kb",
                0x1532,
                0x0298,
                Some(0xFF00),
                Some(1),
                None,
                None,
            ),
            "razer.mouse",
            DeviceKind::Keyboard,
        ),
        (
            ep(
                "hyperx_a2",
                0x03F0,
                0x08BE,
                Some(0xFF13),
                Some(0xFF00),
                None,
                None,
            ),
            "hyperx.alpha2_haste",
            DeviceKind::Headset,
        ),
    ];

    for (endpoint, expected_exact_id, expected_kind) in cases {
        let matches = match_providers(&registry, &endpoint);
        let exact: Vec<&str> = matches
            .iter()
            .filter(|(_, decision)| *decision == MatchDecision::Exact)
            .map(|(p, _)| p.id())
            .collect();
        assert_eq!(exact, vec![expected_exact_id]);

        let selected = matches
            .iter()
            .find(|(p, _)| p.id() == expected_exact_id)
            .expect("expected exact provider present")
            .0
            .device_kind(&endpoint);
        assert_eq!(selected, expected_kind);
    }

    // Family matchers still match unenumerated PIDs in the same vendor/usage_page family.
    let unknown_ajazz = ep("aj_unk", 0x3151, 0x4999, Some(0xFFFF), Some(2), None, None);
    let ajazz_matches = match_providers(&registry, &unknown_ajazz);
    assert_eq!(ajazz_matches.len(), 1);
    assert_eq!(ajazz_matches[0].0.id(), "ajazz.aj_series");
    assert_eq!(ajazz_matches[0].1, MatchDecision::Family);
}

#[test]
fn negative_steelseries_discrete_and_adc_out_of_range_rejected() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // Nova Pro Wireless discrete scale is 0..=8; 0xFF must fail rather than clamp to 100%.
    let mut npw_bad = vec![0_u8; 63];
    npw_bad[0] = 0xB0;
    npw_bad[5] = 0xFF;
    npw_bad[14] = 0x08;
    let npw_oob = make_fixture(
        "steelseries.nova_pro_wireless",
        ep(
            "ss_npw_oob",
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
            step_read(0x06, &npw_bad),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    // Arctis 9 ADC range is 0x64..=0x9A; 0xFF must fail rather than clamp to 100%.
    let arctis9_oob = make_fixture(
        "steelseries.arctis9",
        ep(
            "ss_a9_oob",
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
            step_read(0x00, &[0x20, 0x00, 0x00, 0xFF, 0x00]),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    // Arctis 7+ discrete scale is 0..=4; 0xFF must fail rather than clamp to 100%.
    let nova_7plus_oob = make_fixture(
        "steelseries.nova_family",
        ep(
            "ss_7p_oob",
            0x1038,
            0x220E,
            Some(0xFFC0),
            Some(1),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0x00, &padded(&[0xB0], 63)),
            step_read(0x00, &[0xB0, 0x03, 0xFF, 0x00, 0x00, 0x00, 0x00]),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    verify_all(&registry, &[npw_oob, arctis9_oob, nova_7plus_oob]);
}

#[test]
fn negative_corsair_bragi_tightened_matcher_and_family_priority() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // Non-vendor collections on interface 4 must never match Corsair Bragi.
    for (id, usage_page) in [("cor_if4_gd", 0x0001_u16), ("cor_if4_cc", 0x000C_u16)] {
        let non_vendor = ep(id, 0x1B1C, 0x2B00, Some(usage_page), Some(1), Some(4), None);
        assert!(
            match_providers(&registry, &non_vendor).is_empty(),
            "non-vendor Corsair interface 4 endpoint `{id}` must not match"
        );
    }

    // Known Bragi PID on vendor page -> Exact.
    let known = ep(
        "cor_known",
        0x1B1C,
        0x2B00,
        Some(0xFF42),
        Some(1),
        None,
        None,
    );
    let known_matches = match_providers(&registry, &known);
    assert_eq!(known_matches.len(), 1);
    assert_eq!(known_matches[0].0.id(), "corsair.bragi");
    assert_eq!(known_matches[0].1, MatchDecision::Exact);

    // Unknown Corsair PID on Bragi usage_page 0xFF42 -> Family.
    let unknown = ep("cor_unk", 0x1B1C, 0x2BFF, Some(0xFF42), Some(1), None, None);
    let unknown_matches = match_providers(&registry, &unknown);
    assert_eq!(unknown_matches.len(), 1);
    assert_eq!(unknown_matches[0].0.id(), "corsair.bragi");
    assert_eq!(unknown_matches[0].1, MatchDecision::Family);

    // Excluded Void PID must not match Bragi even if 0xFF42 appears.
    let void_on_bragi = ep(
        "cor_void_42",
        0x1B1C,
        0x0A55,
        Some(0xFF42),
        Some(1),
        None,
        None,
    );
    assert!(match_providers(&registry, &void_on_bragi).is_empty());
}

#[test]
fn logitech_hidpp_scans_slot_6_and_direct_0xff_skipping_hidpp_errors() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // Unifying receiver (0xC52B): slots 1..=5 return HID++ error 0x09 (UnknownDevice), slot 6 answers.
    let mut slot6_steps = Vec::new();
    for slot in 1..=5_u8 {
        slot6_steps.push(step_write(0x11, &padded(&[slot, 0x00, 0x1E], 19)));
        slot6_steps.push(step_read(
            0x11,
            &padded(&[slot, 0xFF, 0x00, 0x1E, 0x09], 19),
        ));
    }
    slot6_steps.push(step_write(0x11, &padded(&[6, 0x00, 0x1E], 19)));
    slot6_steps.push(step_read(0x11, &padded(&[6, 0x00, 0x1E, 0x04, 0x02], 19)));
    slot6_steps.push(step_write(0x11, &padded(&[6, 0x00, 0x0E, 0x10, 0x00], 19)));
    slot6_steps.push(step_read(0x11, &padded(&[6, 0x00, 0x0E, 0x05], 19)));
    slot6_steps.push(step_write(0x11, &padded(&[6, 0x05, 0x0E], 19)));
    slot6_steps.push(step_read(
        0x11,
        &padded(&[6, 0x05, 0x0E, 64, 0x00, 0x00], 19),
    ));

    let receiver_slot6 = make_fixture(
        "logitech.hidpp",
        ep(
            "logi_slot6",
            0x046D,
            0xC52B,
            Some(0xFF00),
            Some(2),
            None,
            Some("Unifying Receiver"),
        ),
        slot6_steps,
        expect_pct(64, ChargeState::Discharging, None),
    );

    // Direct wired/BT Logitech mouse (PID 0xC09B, outside 0xC5xx receiver range): probes 0xFF first.
    let direct_0xff = make_fixture(
        "logitech.hidpp",
        ep(
            "logi_direct_ff",
            0x046D,
            0xC09B,
            Some(0xFF00),
            Some(2),
            None,
            Some("PRO X SUPERLIGHT 2"),
        ),
        vec![
            step_write(0x11, &padded(&[0xFF, 0x00, 0x1E], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x1E, 0x04, 0x02], 19)),
            step_write(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x10, 0x00], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x06], 19)),
            step_write(0x11, &padded(&[0xFF, 0x06, 0x0E], 19)),
            step_read(0x11, &padded(&[0xFF, 0x06, 0x0E, 88, 0x00, 0x01], 19)),
        ],
        expect_pct(88, ChargeState::Charging, None),
    );

    verify_all(&registry, &[receiver_slot6, direct_0xff]);
}

#[test]
fn negative_protocol_family_abnormal_paths() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // 1. CompX (`vxe.r1`) corrupted 0x55-sum checksum -> ChecksumFailed.
    let compx_bad_ck = make_fixture(
        "vxe.r1",
        ep("vxe_bad_ck", 0x373B, 0x10C9, Some(0xFF02), None, None, None),
        vec![
            step_flush_timeout(),
            step_set_output(
                0x08,
                &[0x04, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x49],
            ),
            step_read(
                0x08,
                &[
                    0x04, 0, 0, 0, 0x02, 0x28, 0x00, 0x0E, 0x50, 0, 0, 0, 0, 0, 0, 0x00,
                ],
            ),
        ],
        Expectation::Error {
            error: ErrorKind::ChecksumFailed,
        },
    );

    // 2. Bolutek (`atk.n9v2`) unknown charge state byte -> InvalidResponse.
    let bolutek_bad_charge = make_fixture(
        "atk.n9v2",
        ep("atk_n9_bad", 0x373B, 0x1262, Some(0xFF90), None, None, None),
        vec![
            step_flush_timeout(),
            step_write(0x55, &padded(&[0x65, 0x01], 63)),
            step_read(0x55, &padded(&[0x65, 80, 0x09], 63)),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    // 3. DriverCore (`glorious.mouse`) out-of-range percentage 150 -> InvalidResponse.
    let dc_req = padded(&[0x00, 0x00, 0x02, 0x02, 0x00, 0x83], 64);
    let dc_bad = padded(&[0xA1, 0x00, 0x02, 0x02, 0x00, 0x83, 0x00, 150], 64);
    let drivercore_oob = make_fixture(
        "glorious.mouse",
        ep(
            "glor_oob",
            0x258A,
            0x2022,
            Some(0xFFFF),
            Some(0),
            None,
            None,
        ),
        vec![
            step_send_feature(0x00, &dc_req),
            step_get_feature(0x00, 64, &dc_bad),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    // 4. Razer BlackShark V2 HS corrupted XOR checksum -> ChecksumFailed.
    let mut bs_req = [0_u8; 64];
    bs_req[0] = 0x02;
    bs_req[2] = 0x60;
    bs_req[6] = 0x04;
    bs_req[9] = 0x80;
    bs_req[10] = 0x21;
    bs_req[62] = bs_req[..62].iter().fold(0_u8, |a, b| a ^ b);
    let mut bs_bad = [0_u8; 64];
    bs_bad[0] = 0x02;
    bs_bad[2] = 0x60;
    bs_bad[10] = 0x21;
    bs_bad[12] = 1;
    bs_bad[13] = 75;
    bs_bad[62] = bs_bad[1..62].iter().fold(0_u8, |a, b| a ^ b) ^ 0xFF;
    let razer_bad_xor = make_fixture(
        "razer.blackshark_v2_hs",
        ep(
            "rz_bad_xor",
            0x1532,
            0x0565,
            Some(0xFF14),
            Some(1),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0x02, &bs_req[1..]),
            step_read(0x02, &bs_bad[1..]),
        ],
        Expectation::Error {
            error: ErrorKind::ChecksumFailed,
        },
    );

    // 5. EVision (`evision.atk_keyboard`) out-of-range percentage 150 -> InvalidResponse.
    let evision_oob = make_fixture(
        "evision.atk_keyboard",
        ep(
            "ev_atk_oob",
            0x320F,
            0x5055,
            Some(0xFF1C),
            Some(0x0092),
            None,
            Some("ATK Keyboard"),
        ),
        vec![
            step_flush_timeout(),
            step_write(0x04, &padded(&[0x01, 0x00, 0x1A, 0x00], 63)),
            step_read(
                0x04,
                &padded(&[0x01, 0x00, 0x1A, 0x00, 0x00, 0x00, 0x00, 150, 0x00], 63),
            ),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    verify_all(
        &registry,
        &[
            compx_bad_ck,
            bolutek_bad_charge,
            drivercore_oob,
            razer_bad_xor,
            evision_oob,
        ],
    );
}

#[test]
fn logitech_g930_beats_hidpp10_and_g535_consumer_matches_hidpp() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // logitech.g930 (Exact) beats logitech.hidpp10 (Family) on 046D:0A1F (0xFF00 / 0x0001).
    let g930_ep = ep(
        "logi_g930_prio",
        0x046D,
        0x0A1F,
        Some(0xFF00),
        Some(0x0001),
        None,
        None,
    );
    let g930_matches = match_providers(&registry, &g930_ep);
    let exact: Vec<&str> = g930_matches
        .iter()
        .filter(|(_, d)| *d == MatchDecision::Exact)
        .map(|(p, _)| p.id())
        .collect();
    let family: Vec<&str> = g930_matches
        .iter()
        .filter(|(_, d)| *d == MatchDecision::Family)
        .map(|(p, _)| p.id())
        .collect();
    assert_eq!(exact, vec!["logitech.g930"]);
    assert_eq!(family, vec!["logitech.hidpp10"]);
    let g930_provider = g930_matches
        .iter()
        .find(|(p, _)| p.id() == "logitech.g930")
        .expect("logitech.g930 present")
        .0;
    assert_eq!(g930_provider.device_kind(&g930_ep), DeviceKind::Headset);

    // logitech.hidpp matches G535 (046D:0AC4) on usage_page=0x000C, usage=0x0001, interface=3.
    let g535_ep = ep(
        "logi_g535_cc",
        0x046D,
        0x0AC4,
        Some(0x000C),
        Some(0x0001),
        Some(3),
        None,
    );
    let g535_matches = match_providers(&registry, &g535_ep);
    assert_eq!(g535_matches.len(), 1);
    assert_eq!(g535_matches[0].0.id(), "logitech.hidpp");
    assert_eq!(g535_matches[0].1, MatchDecision::Exact);
    assert_eq!(g535_matches[0].0.device_kind(&g535_ep), DeviceKind::Headset);

    // Wrong interface on G535 consumer control collection must not match.
    let g535_wrong_if = ep(
        "logi_g535_if0",
        0x046D,
        0x0AC4,
        Some(0x000C),
        Some(0x0001),
        Some(0),
        None,
    );
    assert!(match_providers(&registry, &g535_wrong_if).is_empty());
}

#[test]
fn logitech_hidpp20_exact_suppresses_sibling_hidpp10_family_endpoint() {
    use std::sync::Arc;
    use wireless_battery::model::{Endpoint, EndpointId};
    use wireless_battery::transport::{HidSession, Transport, TransportError};
    use wireless_battery::{BatteryClient, ClientConfig};

    struct StaticTransport(Vec<Endpoint>);
    impl Transport for StaticTransport {
        fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
            Ok(self.0.clone())
        }
        fn open(&self, _endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
            Err(TransportError::Timeout)
        }
    }

    let gpw3_long = ep(
        "gpw3_long",
        0x046D,
        0xC54D,
        Some(0xFF00),
        Some(0x0002),
        Some(2),
        Some("PRO X 2"),
    );
    let gpw3_short = ep(
        "gpw3_short",
        0x046D,
        0xC54D,
        Some(0xFF00),
        Some(0x0001),
        Some(2),
        Some("PRO X 2"),
    );
    let legacy_hidpp10 = ep(
        "legacy_hidpp10",
        0x046D,
        0xC52F,
        Some(0xFF00),
        Some(0x0001),
        Some(1),
        Some("Logitech Nano Receiver"),
    );

    let transport = Arc::new(StaticTransport(vec![gpw3_long, gpw3_short, legacy_hidpp10]));
    let client = BatteryClient::with_transport(
        ClientConfig::default(),
        transport,
        Registry::builtin().expect("builtin registry"),
    )
    .expect("client must initialize");
    let devices = client.list_devices().expect("list_devices must succeed");
    let readable: Vec<(&str, Option<&str>)> = devices
        .iter()
        .filter(|d| d.readable)
        .map(|d| (d.endpoint_id.0.as_str(), d.provider_id.as_deref()))
        .collect();
    assert_eq!(
        readable,
        vec![
            ("ep_gpw3_long", Some("logitech.hidpp")),
            ("ep_legacy_hidpp10", Some("logitech.hidpp10")),
        ]
    );
}

#[test]
fn logitech_centurion_direct_vs_bridged_and_offline_state() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    let bridged_ep = ep(
        "cent_bridged",
        0x046D,
        0x0AF7,
        Some(0xFFA0),
        Some(0x0001),
        Some(3),
        None,
    );
    let direct_ep = ep(
        "cent_direct",
        0x046D,
        0x0B2E,
        Some(0xFFA0),
        Some(0x0001),
        Some(3),
        None,
    );
    for endpoint in [&bridged_ep, &direct_ep] {
        let matches = match_providers(&registry, endpoint);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0.id(), "logitech.centurion");
        assert_eq!(matches[0].1, MatchDecision::Exact);
    }

    // Direct USB-wired Centurion (0x0B2E, prefix [0x50, 0x23]) queries root -> battery directly.
    let direct_ok = make_fixture(
        "logitech.centurion",
        direct_ep,
        vec![
            step_write(0x50, &padded(&[0x23, 5, 0x00, 0x00, 0x01, 0x01, 0x04], 63)),
            step_read(0x50, &padded(&[0x23, 4, 0x00, 0x00, 0x01, 0x05], 63)),
            step_write(0x50, &padded(&[0x23, 3, 0x00, 0x05, 0x01], 63)),
            step_read(
                0x50,
                &padded(&[0x23, 6, 0x00, 0x05, 0x01, 64, 0x00, 0x00], 63),
            ),
        ],
        expect_pct(64, ChargeState::Discharging, None),
    );

    // Bridged Centurion (0x0AF7) reports Offline when bridge connection event signals disconnected.
    let bridged_offline = make_fixture(
        "logitech.centurion",
        bridged_ep,
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
            step_read(0x51, &padded(&[5, 0x00, 0x03, 0x00, 0x00, 0x00], 63)),
        ],
        Expectation::Reading {
            percentage: None,
            level: None,
            voltage_mv: None,
            charge_state: ChargeState::Unknown,
            reachability: Reachability::Offline,
            precision: Precision::None,
        },
    );

    verify_all(&registry, &[direct_ok, bridged_offline]);
}

#[test]
fn corsair_void_0x64_header_and_bragi_headset_kind() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // corsair.void accepts real 0x64 response header as well as 0xCA.
    let void_64 = make_fixture(
        "corsair.void",
        ep(
            "cor_void_64",
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
            step_read(0x64, &[0x00, 0x80 | 68, 0x00, 0x01]),
        ],
        expect_pct(68, ChargeState::Discharging, None),
    );
    verify_fixture(&void_64, registry.clone())
        .expect("corsair.void must accept 0x64 response header");

    // corsair.bragi classifies 0x2A08 as DeviceKind::Headset with Exact match.
    let bragi_hs = ep(
        "cor_bragi_2a08",
        0x1B1C,
        0x2A08,
        Some(0xFF42),
        Some(1),
        None,
        None,
    );
    let matches = match_providers(&registry, &bragi_hs);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0.id(), "corsair.bragi");
    assert_eq!(matches[0].1, MatchDecision::Exact);
    assert_eq!(matches[0].0.device_kind(&bragi_hs), DeviceKind::Headset);
}

#[test]
fn sony_pulse_3d_charging_and_offline_states() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // Charging: r[3] == 0x80, online bit (r[4] & 0x08) != 0 -> percentage: None, ChargeState::Charging.
    let pulse_charging = make_fixture(
        "sony.pulse_3d",
        ep(
            "pulse3d_chg",
            0x054C,
            0x0D5E,
            Some(0xFF01),
            Some(0x0020),
            None,
            None,
        ),
        vec![
            step_read(0xB0, &[0x00, 0x00, 0x80, 0x08, 0x00, 0x00, 0x00]),
            step_flush_timeout(),
        ],
        Expectation::Reading {
            percentage: None,
            level: None,
            voltage_mv: None,
            charge_state: ChargeState::Charging,
            reachability: Reachability::Online,
            precision: Precision::None,
        },
    );

    // Offline: (r[4] & 0x08) == 0 -> Reachability::Offline, percentage: None.
    let pulse_offline = make_fixture(
        "sony.pulse_3d",
        ep(
            "pulse3d_off",
            0x054C,
            0x0D5E,
            Some(0xFF01),
            Some(0x0020),
            None,
            None,
        ),
        vec![
            step_read(0xB0, &[0x00, 0x00, 50, 0x00, 0x00, 0x00, 0x00]),
            step_flush_timeout(),
        ],
        Expectation::Reading {
            percentage: None,
            level: None,
            voltage_mv: None,
            charge_state: ChargeState::Unknown,
            reachability: Reachability::Offline,
            precision: Precision::None,
        },
    );

    verify_all(&registry, &[pulse_charging, pulse_offline]);
}

#[test]
fn negative_new_headset_providers_reject_out_of_range_percentage() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    let lenovo_oob = make_fixture(
        "lenovo.wireless_voip",
        ep(
            "lenovo_oob",
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
            step_read(0x27, &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 101]),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    let fifine_oob = make_fixture(
        "fifine.ht6",
        ep(
            "fifine_oob",
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
            step_read(0x55, &[0x65, 101, 0x00]),
        ],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );

    verify_all(&registry, &[lenovo_oob, fifine_oob]);
}

#[test]
fn apple_magic_ignores_generic_keyboard_and_rejects_out_of_range() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // Generic desktop keyboard collection (0x0001) on 004C:026C must not match apple.magic.
    let generic_kb = ep(
        "apple_generic_kb",
        0x004C,
        0x026C,
        Some(0x0001),
        Some(0x0006),
        None,
        None,
    );
    assert!(match_providers(&registry, &generic_kb).is_empty());

    // Out-of-range percentage (101) must be rejected as InvalidResponse.
    let magic_oob = make_fixture(
        "apple.magic",
        ep(
            "apple_magic_oob",
            0x004C,
            0x026C,
            Some(0xFF00),
            Some(1),
            None,
            None,
        ),
        vec![step_get_input(
            0x90,
            7,
            &[0x00, 101, 0x00, 0x00, 0x00, 0x00, 0x00],
        )],
        Expectation::Error {
            error: ErrorKind::InvalidResponse,
        },
    );
    verify_fixture(&magic_oob, registry)
        .expect("apple.magic must reject out-of-range percentage 101");
}

#[test]
fn atk_zero_and_ninjutso_sora_v2_report_sleeping() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    let sleeping = Expectation::Reading {
        percentage: None,
        level: None,
        voltage_mv: None,
        charge_state: ChargeState::Unknown,
        reachability: Reachability::Sleeping,
        precision: Precision::None,
    };

    // atk.zero reports Reachability::Sleeping when r[2] == 0x01.
    let atk_sleeping = make_fixture(
        "atk.zero",
        ep(
            "atk_zero_sleep",
            0x373B,
            0x124F,
            Some(0xFF05),
            Some(0x0001),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(
                0x08,
                &padded(&[0x7D, 0x72, 0x02, 0x00, 0x01, 0x07, 0x01], 63),
            ),
            step_read(0x08, &[0x72, 0x01, 0x00, 0x00, 0x07]),
        ],
        sleeping.clone(),
    );

    // ninjutso.sora_v2 reports Reachability::Sleeping when connected (r[12] == 1) and r[9] == 0 && r[11] == 0.
    let mut sora_sleep_resp = vec![0_u8; 31];
    sora_sleep_resp[0] = 0x15; // r[1]
    sora_sleep_resp[8] = 0; // r[9] = 0%
    sora_sleep_resp[10] = 0; // r[11] = not full
    sora_sleep_resp[11] = 1; // r[12] = connected
    let sora_sleeping = make_fixture(
        "ninjutso.sora_v2",
        ep(
            "sora_v2_sleep",
            0x1915,
            0xAE1C,
            Some(0xFFA0),
            Some(0x0001),
            None,
            None,
        ),
        vec![
            step_send_feature(0x05, &padded(&[0x15, 0x00, 0x00, 0x01], 31)),
            step_get_feature(0x05, 31, &sora_sleep_resp),
        ],
        sleeping,
    );

    verify_all(&registry, &[atk_sleeping, sora_sleeping]);
}

#[test]
fn attack_shark_x6_scaling_and_steelseries_new_pids() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // xenta.mouse on 1D57:FA62 (model 0x85): level 8 scales to 80% when discharging and 0x80 is Charging.
    let x6_discharging = make_fixture(
        "xenta.mouse",
        ep("x6_dis", 0x1D57, 0xFA62, Some(0x000A), None, None, None),
        vec![step_read(0x03, &[0x85, 0x40, 0x01, 8])],
        expect_pct(80, ChargeState::Discharging, None),
    );
    let x6_charging = make_fixture(
        "xenta.mouse",
        ep("x6_chg", 0x1D57, 0xFA62, Some(0x000A), None, None, None),
        vec![step_read(0x03, &[0x85, 0x40, 0x80, 8])],
        expect_pct(80, ChargeState::Charging, None),
    );
    verify_all(&registry, &[x6_discharging, x6_charging]);

    // steelseries.aerox matches new Aerox 9 WoW (0x1874/0x1876) and Prime Mini Wireless (0x1848/0x184A) PIDs.
    for pid in [0x1874_u16, 0x1876, 0x1848, 0x184A] {
        let endpoint = ep(
            &format!("ss_aerox_{pid:04x}"),
            0x1038,
            pid,
            Some(0xFF00),
            Some(1),
            Some(3),
            None,
        );
        let matches = match_providers(&registry, &endpoint);
        assert_eq!(matches.len(), 1, "expected exact match for PID 0x{pid:04X}");
        assert_eq!(matches[0].0.id(), "steelseries.aerox");
        assert_eq!(matches[0].1, MatchDecision::Exact);
    }
}

#[test]
fn apple_magic_mouse_and_trackpad_device_kinds() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    let cases = [
        ("apple_mm2", 0x004C_u16, 0x0269_u16, DeviceKind::Mouse),
        ("apple_mm_usbc", 0x05AC, 0x0323, DeviceKind::Mouse),
        ("apple_mt2", 0x004C, 0x0265, DeviceKind::Peripheral),
        ("apple_mt_usbc", 0x05AC, 0x0324, DeviceKind::Peripheral),
    ];

    for (id, vid, pid, expected_kind) in cases {
        let endpoint = ep(id, vid, pid, Some(0xFF00), Some(1), None, None);
        let matches = match_providers(&registry, &endpoint);
        assert_eq!(
            matches.len(),
            1,
            "expected exact match for {vid:04X}:{pid:04X}"
        );
        assert_eq!(matches[0].0.id(), "apple.magic");
        assert_eq!(matches[0].1, MatchDecision::Exact);
        assert_eq!(matches[0].0.device_kind(&endpoint), expected_kind);
    }
}

#[test]
fn sony_ds4_error_codes_and_dongle_offline_bit() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // DS4 cable + level 14 (error code) -> (None, ChargeState::Unknown), Online.
    let mut ds4_err = vec![0_u8; 63];
    ds4_err[29] = 0x10 | 14;
    let ds4_error_fixture = make_fixture(
        "sony.dualshock4",
        ep("ds4_err", 0x054C, 0x09CC, Some(1), Some(5), None, None),
        vec![step_read(0x01, &ds4_err)],
        Expectation::Reading {
            percentage: None,
            level: None,
            voltage_mv: None,
            charge_state: ChargeState::Unknown,
            reachability: Reachability::Online,
            precision: Precision::None,
        },
    );

    // DS4 wireless adapter (0x0BA0) with frame[31] & 0x04 != 0 -> Reachability::Offline.
    let mut ds4_dongle = vec![0_u8; 63];
    ds4_dongle[29] = 0x05; // non-zero status byte
    ds4_dongle[30] = 0x04; // frame[31] dongle offline bit
    let ds4_dongle_offline = make_fixture(
        "sony.dualshock4",
        ep(
            "ds4_dongle_off",
            0x054C,
            0x0BA0,
            Some(1),
            Some(5),
            None,
            None,
        ),
        vec![step_read(0x01, &ds4_dongle)],
        Expectation::Reading {
            percentage: None,
            level: None,
            voltage_mv: None,
            charge_state: ChargeState::Unknown,
            reachability: Reachability::Offline,
            precision: Precision::None,
        },
    );

    verify_all(&registry, &[ds4_error_fixture, ds4_dongle_offline]);
}

#[test]
fn nintendo_switch_extra_pids_and_host_powered_full() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // Joy-Con (L) (057E:2006) and HORI Wireless Switch Pad (0F0D:00F6) match nintendo.switch_pro.
    for (id, vid, pid) in [
        ("joycon_l", 0x057E_u16, 0x2006_u16),
        ("hori_pad", 0x0F0D, 0x00F6),
    ] {
        let endpoint = ep(id, vid, pid, Some(1), Some(5), None, None);
        let matches = match_providers(&registry, &endpoint);
        assert_eq!(matches.len(), 1, "expected exact match for {id}");
        assert_eq!(matches[0].0.id(), "nintendo.switch_pro");
        assert_eq!(matches[0].1, MatchDecision::Exact);
    }

    // raw_level == 4 with host_powered (0x01) and charging == 0 reports ChargeState::Full.
    let mut sp_full = vec![0_u8; 48];
    sp_full[1] = (4 << 5) | 0x01;
    let host_powered_full = make_fixture(
        "nintendo.switch_pro",
        ep("switch_full", 0x057E, 0x2006, Some(1), Some(5), None, None),
        vec![step_read(0x30, &sp_full)],
        Expectation::Reading {
            percentage: None,
            level: Some(BatteryLevel::Full),
            voltage_mv: None,
            charge_state: ChargeState::Full,
            reachability: Reachability::Online,
            precision: Precision::DiscreteLevel,
        },
    );

    verify_fixture(&host_powered_full, registry)
        .expect("host-powered Switch controller at level 4 must report Full");
}

#[test]
fn corsair_void_critical_and_full_states() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // PIDs 0x0A0C and 0x1B25 match corsair.void.
    for pid in [0x0A0C_u16, 0x1B25] {
        let endpoint = ep(
            &format!("cor_void_{pid:04x}"),
            0x1B1C,
            pid,
            Some(0xFFC5),
            Some(1),
            None,
            None,
        );
        let matches = match_providers(&registry, &endpoint);
        assert_eq!(matches.len(), 1, "expected exact match for 1B1C:{pid:04X}");
        assert_eq!(matches[0].0.id(), "corsair.void");
        assert_eq!(matches[0].1, MatchDecision::Exact);
    }

    // st == 3 -> Discharging with valid percentage.
    let void_critical = make_fixture(
        "corsair.void",
        ep(
            "cor_void_crit",
            0x1B1C,
            0x0A0C,
            Some(0xFFC5),
            Some(1),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0xC9, &padded(&[0x64], 63)),
            step_read(0xCA, &[0x00, 0x80 | 12, 0x00, 0x03]),
        ],
        expect_pct(12, ChargeState::Discharging, None),
    );

    // st == 4 -> ChargeState::Full with valid percentage.
    let void_full = make_fixture(
        "corsair.void",
        ep(
            "cor_void_full",
            0x1B1C,
            0x1B25,
            Some(0xFFC5),
            Some(1),
            None,
            None,
        ),
        vec![
            step_flush_timeout(),
            step_write(0xC9, &padded(&[0x64], 63)),
            step_read(0xCA, &[0x00, 0x80 | 100, 0x00, 0x04]),
        ],
        expect_pct(100, ChargeState::Full, None),
    );

    verify_all(&registry, &[void_critical, void_full]);
}

#[test]
fn logitech_hidpp_1004_error_and_1f20_full() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    // 0x1004 UnifiedBattery status 4 -> ChargeState::Unknown (NOT Charging).
    let hidpp_1004_err = make_fixture(
        "logitech.hidpp",
        ep(
            "logi_1004_err",
            0x046D,
            0xC09B,
            Some(0xFF00),
            Some(2),
            None,
            Some("PRO X SUPERLIGHT 2"),
        ),
        vec![
            step_write(0x11, &padded(&[0xFF, 0x00, 0x1E], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x1E, 0x04, 0x02], 19)),
            step_write(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x10, 0x00], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x00], 19)),
            step_write(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x10, 0x04], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x05], 19)),
            step_write(0x11, &padded(&[0xFF, 0x05, 0x1E], 19)),
            step_read(0x11, &padded(&[0xFF, 0x05, 0x1E, 50, 0x00, 0x04], 19)),
        ],
        expect_pct(50, ChargeState::Unknown, None),
    );

    // 0x1F20 ADC status 0x07 -> ChargeState::Full.
    let mv_full = 4102_u16.to_be_bytes();
    let hidpp_1f20_full = make_fixture(
        "logitech.hidpp",
        ep(
            "logi_1f20_full",
            0x046D,
            0x0AAA,
            Some(0xFF00),
            Some(2),
            None,
            Some("Logitech G PRO X"),
        ),
        vec![
            step_write(0x11, &padded(&[0xFF, 0x00, 0x1E], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x1E, 0x04, 0x02], 19)),
            step_write(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x10, 0x00], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x00], 19)),
            step_write(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x10, 0x04], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x00], 19)),
            step_write(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x10, 0x01], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x00], 19)),
            step_write(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x1F, 0x20], 19)),
            step_read(0x11, &padded(&[0xFF, 0x00, 0x0E, 0x07], 19)),
            step_write(0x11, &padded(&[0xFF, 0x07, 0x0E], 19)),
            step_read(
                0x11,
                &padded(&[0xFF, 0x07, 0x0E, mv_full[0], mv_full[1], 0x07], 19),
            ),
        ],
        expect_pct(100, ChargeState::Full, Some(4102)),
    );

    verify_all(&registry, &[hidpp_1004_err, hidpp_1f20_full]);
}
