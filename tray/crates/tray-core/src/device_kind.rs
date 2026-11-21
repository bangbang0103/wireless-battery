//! Conservative hardware category classification for UI icon selection.

use serde::{Deserialize, Serialize};
use wireless_battery::{Device, TransportKind};

/// Coarse hardware silhouette rendered in the tray popover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Mouse,
    Keyboard,
    Gamepad,
    Headset,
    Peripheral,
}

/// Infer a UI icon category from provider ID, transport, and display name.
#[must_use]
pub fn classify_device(device: &Device) -> DeviceKind {
    if device.endpoint.transport == TransportKind::XInput {
        return DeviceKind::Gamepad;
    }

    if let Some(provider) = device.provider_id.as_deref() {
        let p = provider.to_ascii_lowercase();
        if p.contains("headset")
            || p.contains("xiberia")
            || p.contains("atk.n9v2")
            || p.contains("mchose.v9")
            || p.contains("vgn.omnisound")
            || p.contains("arctis")
            || p.contains("nova_")
            || p.contains("corsair.void")
            || p.contains("hyperx.cloud")
            || p.contains("hyperx.stinger")
            || p.contains("audeze.")
            || p.contains("endorfy.")
            || p.contains("akg.")
            || p.contains("turtlebeach.")
            || p.contains("jabra.")
            || p.contains("jbl.")
            || p.contains("plantronics.")
            || p.contains("fractal.")
            || p.contains("alienware.")
            || p.contains("moondrop.")
            || p.contains("astro.")
            || p.contains("logitech.g435")
            || p.contains("sony.inzone")
            || p.contains("asus.rog_cetra")
            || p.contains("razer.blackshark")
            || p.contains("razer.barracuda")
            || p.contains("darkrim.")
        {
            return DeviceKind::Headset;
        }
        if p.contains("dualsense")
            || p.contains("dualshock")
            || p.contains("xinput")
            || p.contains("gamepad")
            || p.contains("switch_pro")
            || p.contains("steam_controller")
            || p.contains("gamesir.")
            || p.contains("flydigi.")
            || p.contains("eightbitdo.")
            || p.contains("beitong.")
        {
            return DeviceKind::Gamepad;
        }
        if p.contains("keyboard")
            || p.contains("rainy75")
            || p.contains("dareu.a84")
            || p.contains("cherry.mx")
            || p.contains("akko.dash")
            || p.contains("nuphy.")
            || p.contains("atk.z87")
            || p.contains("ajazz_ak992")
            || p.contains("aula.hero68")
            || p.contains("via.dongle")
            || p.contains("mchose.k87")
        {
            return DeviceKind::Keyboard;
        }
        if p.contains("mouse")
            || p.contains("vxe.r1")
            || p.contains("a950")
            || p.contains("dareu.togran")
            || p.contains("zenith.")
            || p.contains("a7_ultra")
            || p.contains("a5_gen1")
            || p.contains("g3_v2")
            || p.contains("k7_v2")
            || p.contains("eweadn")
            || p.contains("rapoo.")
            || p.contains("atk.")
            || p.contains("imecoo.")
            || p.contains("crociris.")
            || p.contains("kysona.")
            || p.contains("chimera.")
            || p.contains("cherry.xtrfy")
            || p.contains("corsair.compx")
            || p.contains("steelseries.aerox")
            || p.contains("steelseries.rival")
            || p.contains("glorious.")
            || p.contains("hitscan.")
            || p.contains("gwolves.")
            || p.contains("arbiter.")
            || p.contains("lamzu.")
            || p.contains("wlmouse.")
            || p.contains("ninjutso.")
            || p.contains("sprime.")
            || p.contains("pulsar.")
            || p.contains("vaxee.")
            || p.contains("endgame.")
            || p.contains("finalmouse.")
            || p.contains("scyrox.")
            || p.contains("rawm.")
            || p.contains("zowie.")
            || p.contains("coolermaster.")
            || p.contains("nzxt.")
            || p.contains("msi.")
            || p.contains("fantech.")
            || p.contains("raigor.")
            || p.contains("sinowealth.")
            || p.contains("royal_kludge.")
            || p.contains("darmoshark.")
            || p.contains("machenike.")
            || p.contains("niceboy.")
            || p.contains("motospeed.")
            || p.contains("xenta.")
            || p.contains("delux.")
            || p.contains("ajazz.aj")
            || p.contains("incott.")
            || p.contains("rongyuan.")
            || p.contains("keychron.nape")
            || p.contains("keychron.ultralink")
            || p.contains("aula.sc580")
            || p.contains("redragon_m999")
            || p.contains("pressplay.")
            || p.contains("hyperx.alpha2_haste")
        {
            if p == "hyperx.alpha2_haste" && device.endpoint.pid == Some(0x08BE) {
                return DeviceKind::Headset;
            }
            return DeviceKind::Mouse;
        }
    }

    let name = device.name.to_ascii_lowercase();
    if name.contains("keyboard")
        || name.contains("rainy75")
        || name.contains("z75")
        || name.contains("azoth")
        || name.contains("falchion")
        || name.contains("mx keys")
        || name.contains("g915")
        || name.contains("g715")
    {
        return DeviceKind::Keyboard;
    }
    if name.contains("headset")
        || name.contains("headphone")
        || name.contains("delta")
        || name.contains("pelta")
        || name.contains("blackshark")
        || name.contains("barracuda")
        || name.contains("arctis")
        || name.contains("virtuoso")
        || name.contains("hs80")
        || name.contains("cloud")
        || name.contains("earbud")
    {
        return DeviceKind::Headset;
    }
    if name.contains("controller")
        || name.contains("dualsense")
        || name.contains("dualshock")
        || name.contains("gamepad")
        || name.contains("xbox")
    {
        return DeviceKind::Gamepad;
    }
    if name.contains("mouse")
        || name.contains("gpw")
        || name.contains("superlight")
        || name.contains("viper")
        || name.contains("deathadder")
        || name.contains("basilisk")
        || name.contains("harpe")
        || name.contains("gladius")
        || name.contains("keris")
        || name.contains("mx master")
        || name.contains("mx anywhere")
    {
        return DeviceKind::Mouse;
    }

    DeviceKind::Peripheral
}

#[cfg(test)]
mod tests {
    use super::*;
    use wireless_battery::{Capabilities, DeviceId, Endpoint, MatchStatus, SupportStatus};

    fn fake_device(name: &str, provider: Option<&str>, transport: TransportKind) -> Device {
        let mut endpoint = Endpoint::synthetic("ep", 0x046D, 0xC547);
        endpoint.transport = transport;
        Device {
            id: DeviceId("dev".to_owned()),
            endpoint_id: endpoint.id.clone(),
            endpoint,
            name: name.to_owned(),
            vendor: None,
            model: None,
            provider_id: provider.map(str::to_owned),
            provider_version: None,
            support_status: SupportStatus::Verified,
            match_status: MatchStatus::Exact,
            capabilities: Capabilities::default(),
            readable: true,
            receiver_id: None,
        }
    }

    #[test]
    fn classifies_known_providers_and_names() {
        assert_eq!(
            classify_device(&fake_device(
                "WOB Rainy75",
                Some("wob.rainy75"),
                TransportKind::Hid
            )),
            DeviceKind::Keyboard
        );
        assert_eq!(
            classify_device(&fake_device(
                "Logitech GPW3",
                Some("logitech.hidpp"),
                TransportKind::Hid
            )),
            DeviceKind::Mouse
        );
        assert_eq!(
            classify_device(&fake_device(
                "XInput slot 0",
                Some("os.xinput"),
                TransportKind::XInput
            )),
            DeviceKind::Gamepad
        );
        assert_eq!(
            classify_device(&fake_device(
                "ROG Delta II",
                Some("asus.headset"),
                TransportKind::Hid
            )),
            DeviceKind::Headset
        );
        assert_eq!(
            classify_device(&fake_device(
                "Arctis Nova 7",
                Some("steelseries.nova_family"),
                TransportKind::Hid
            )),
            DeviceKind::Headset
        );
        assert_eq!(
            classify_device(&fake_device(
                "Keychron Q1 Pro",
                Some("keychron.qmk_keyboard"),
                TransportKind::Hid
            )),
            DeviceKind::Keyboard
        );
        assert_eq!(
            classify_device(&fake_device(
                "Nintendo Switch Pro",
                Some("nintendo.switch_pro"),
                TransportKind::Hid
            )),
            DeviceKind::Gamepad
        );
        assert_eq!(
            classify_device(&fake_device(
                "Lamzu Maya",
                Some("lamzu.mouse"),
                TransportKind::Hid
            )),
            DeviceKind::Mouse
        );
        let mut alpha2 = fake_device(
            "HyperX Cloud Alpha 2",
            Some("hyperx.alpha2_haste"),
            TransportKind::Hid,
        );
        alpha2.endpoint.pid = Some(0x08BE);
        assert_eq!(classify_device(&alpha2), DeviceKind::Headset);
    }
}
