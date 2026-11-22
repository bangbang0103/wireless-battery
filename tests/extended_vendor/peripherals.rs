//! Replay tests for Razer headsets, ASUS ROG, CompX, Dareu, VGN, Keychron, keyboards, and MCHOSE.

use super::common::*;
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::Fixture;
use wireless_battery::ChargeState;

pub(super) fn razer_and_asus_fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();

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
    bs2020[10] = 0x00;
    bs2020[11..13].copy_from_slice(&4000_u16.to_be_bytes());
    bs2020[13] = 63;
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
        r[0] = 0x02;
        r[1] = txid;
        r[5] = 2;
        r[6] = 0x07;
        r[7] = cmd;
        r[9] = val;
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
    v3_bat[12] = 54;
    v3_bat[13] = 1;
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
    rog_kb[5] = 88;
    rog_kb[8] = 1;
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

    fixtures
}

pub(super) fn compx_dareu_vgn_fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();

    let compx_frame = |cmd: u8, b5: u8, b6: u8, b7: u8| {
        let mut f = [0_u8; 17];
        f[0] = 0x08;
        f[1] = cmd;
        f[5] = b5;
        f[6] = b6;
        f[7] = b7;
        f[8] = 0x0F;
        f[9] = 0xA0;
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
    tog[6] = 1;
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
    a84[4] = 89;
    a84[14] = 1;
    fixtures.push(make_fixture(
        "dareu.a84",
        ep("dareu_a84", 0x260D, 0x0022, Some(1), Some(6), None, None),
        vec![
            step_send_feature(0x00, &padded(&[0x04, 0xF6], 64)),
            step_get_feature(0x00, 64, &a84),
        ],
        expect_pct(89, ChargeState::Unknown, None),
    ));

    fixtures
}

pub(super) fn keyboards_and_mchose_fixtures() -> Vec<Fixture> {
    let mut fixtures = Vec::new();

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
    sc580[12] = 0;
    sc580[13] = 73;
    sc580[14] = 1;
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

    fixtures
}

#[test]
fn replay_razer_and_asus() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &razer_and_asus_fixtures());
}

#[test]
fn replay_compx_dareu_and_vgn() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &compx_dareu_vgn_fixtures());
}

#[test]
fn replay_keyboards_and_mchose() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");
    verify_all(&registry, &keyboards_and_mchose_fixtures());
}
