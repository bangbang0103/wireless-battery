//! Negative, abnormal-path, tightened-matcher, and Family-vs-Exact priority tests.

use super::common::*;
use wireless_battery::providers::MatchDecision;
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::{verify_fixture, Expectation};
use wireless_battery::{ChargeState, DeviceKind, ErrorKind, Precision, Reachability};

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
