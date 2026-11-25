//! Extended vendor replay tests split by device group plus full builtin coverage assertion.

mod common;
mod console_headsets;
mod gaming_mice;
mod negative_tests;
mod peripherals;

use common::{ep, match_providers};
use std::collections::HashSet;
use wireless_battery::registry::Registry;

#[test]
fn all_builtin_providers_have_replay_coverage() {
    let registry = Registry::builtin().expect("builtin registry must load cleanly");

    let mut tested_ids = HashSet::new();
    for fixture in console_headsets::sony_and_gamepad_fixtures()
        .into_iter()
        .chain(console_headsets::steelseries_and_corsair_fixtures())
        .chain(console_headsets::hyperx_and_logitech_audio_fixtures())
        .chain(console_headsets::headsets_audio_fixtures())
        .chain(peripherals::razer_and_asus_fixtures())
        .chain(peripherals::compx_dareu_vgn_fixtures())
        .chain(peripherals::keyboards_and_mchose_fixtures())
        .chain(gaming_mice::drivercore_and_esports_fixtures())
        .chain(gaming_mice::enthusiast_mice_fixtures())
        .chain(gaming_mice::rapoo_and_misc_fixtures())
    {
        tested_ids.insert(fixture.target.id);
    }

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
    assert_eq!(registry.providers().len(), 163);

    // Quick sanity check verifying an unknown endpoint matches no built-in providers.
    let unknown_ep = ep("unmatched", 0xFFFF, 0xFFFF, None, None, None, None);
    assert!(match_providers(&registry, &unknown_ep).is_empty());
}
