//! Replay tests for DriverCore, esports mice, enthusiast mice/receivers, Rapoo, PressPlay, MAONO, DARKRIM.

use super::common::*;
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::Fixture;
use wireless_battery::ChargeState;

pub(super) fn drivercore_and_esports_fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();

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
    n05_resp[8] = 66;
    n05_resp[9] = 1;
    n05_resp[11] = 1;
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
        r[7] = v;
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
    eg_resp[15] = 72;
    fixtures.push(make_fixture(
        "endgame.4k",
        ep("endgame", 0x3367, 0x1970, Some(0xFF01), Some(2), None, None),
        vec![
            step_send_feature(0xA1, &padded(&[0xB4], 63)),
            step_get_feature(0xA1, 63, &eg_resp),
        ],
        expect_pct(72, ChargeState::Unknown, None),
    ));

    fixtures
}

pub(super) fn enthusiast_mice_fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();

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
    scy_resp[2] = 1;
    scy_resp[6] = 4;
    scy_resp[19] = 69;
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
            step_read(0x05, &[0x00, 0xFA, 0x48, 0xE8, 0x03, 0x01]),
        ],
        expect_pct(90, ChargeState::Charging, Some(4125)),
    ));
    let mut ms_resp = vec![0_u8; 21];
    ms_resp[19] = 128 + 65;
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
    delux[5] = 1;
    delux[17] = 74;
    delux[18] = 0;
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

    fixtures
}

pub(super) fn rapoo_and_misc_fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();

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

    fixtures
}

#[test]
fn replay_drivercore_and_esports_mice() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &drivercore_and_esports_fixtures());
}

#[test]
fn replay_enthusiast_mice_and_receivers() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &enthusiast_mice_fixtures());
}

#[test]
fn replay_rapoo_and_single_model_peripherals() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &rapoo_and_misc_fixtures());
}
