//! Wireless gaming mouse HID battery protocols:
//! DriverCore 0x83 family (Glorious, Hitscan, G-Wolves, Arbiter Studio, LAMZU, WLMouse),
//! Nordic 0x05/0x15 family (Ninjutso Sora V2, Sprime PM1), Ninjutso Ten, Pulsar X2 V3,
//! VAXEE 4K/8K, and Endgame Gear 4K.

use crate::error::BatteryError;
use crate::model::{ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource};
use crate::providers::fixed::{
    capabilities, checked_percentage, experimental_metadata, feature_frame, reading,
    send_with_modes, sleep, SendMode,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(DriverCoreProvider::glorious()),
        Arc::new(DriverCoreProvider::hitscan()),
        Arc::new(DriverCoreProvider::gwolves()),
        Arc::new(DriverCoreProvider::arbiter()),
        Arc::new(DriverCoreProvider::lamzu()),
        Arc::new(DriverCoreProvider::wlmouse()),
        Arc::new(Nordic05Provider::ninjutso_sora_v2()),
        Arc::new(Nordic05Provider::sprime_pm1()),
        Arc::new(NinjutsoTenProvider),
        Arc::new(PulsarX2V3Provider),
        Arc::new(VaxeeProvider),
        Arc::new(Endgame4kProvider),
    ]
}

const GLORIOUS_PIDS: &[u16] = &[
    0x2011, 0x2012, 0x2013, 0x2014, 0x201A, 0x2022, 0x2023, 0x2024, 0x2025, 0x2033, 0x2034, 0x2036,
];
const GLORIOUS_CABLE_PIDS: &[u16] = &[0x2011, 0x2012, 0x2013, 0x2014, 0x201A];

const HITSCAN_DRIVERCORE_PIDS: &[u16] = &[0x0136, 0x0194];

const GWOLVES_CABLE_PIDS: &[u16] = &[
    0x1619, 0x2119, 0x2619, 0x2708, 0x2719, 0x3508, 0x3519, 0x3608, 0x3619, 0x3708, 0x3808, 0x3908,
    0x4219, 0x4718, 0x4719, 0x5219, 0x5308, 0x5404, 0x5408, 0x5418, 0x5419, 0x5608, 0x5618, 0x5619,
    0x5704, 0x5708, 0x5804, 0x5808, 0x5904, 0x5908, 0x6808, 0x7204, 0x7704, 0x7708, 0x7904, 0x7908,
];

const ARBITER_PIDS: &[u16] = &[0x0009, 0x000A, 0x0038, 0x0039, 0x0066, 0x0067];
const ARBITER_CABLE_PIDS: &[u16] = &[0x0009, 0x0038, 0x0066];

const LAMZU_373E_PIDS: &[u16] = &[0x0016, 0x001C, 0x001D, 0x001E];
const LAMZU_37B0_CABLE_PIDS: &[u16] = &[
    0x0005, 0x0007, 0x0009, 0x0011, 0x0017, 0x001C, 0x001F, 0x0021, 0x0025, 0x0028, 0x002C, 0x0030,
    0x0034, 0x0038, 0x003A, 0x003C, 0x003E, 0x0040, 0x0042, 0x0044, 0x0046, 0x0050, 0x0052, 0x0056,
];

const WLMOUSE_PIDS: &[u16] = &[
    0xA856, 0xA860, 0xA861, 0xA866, 0xA867, 0xA868, 0xA870, 0xA871, 0xA874, 0xA878, 0xA879, 0xA880,
    0xA883, 0xA884, 0xA885,
];
const WLMOUSE_CABLE_PIDS: &[u16] = &[0xA861, 0xA866, 0xA867, 0xA871, 0xA879, 0xA884];

#[derive(Clone, Copy)]
enum DriverCoreBrand {
    Glorious,
    Hitscan,
    GWolves,
    Arbiter,
    Lamzu,
    WLMouse,
}

struct DriverCoreProvider {
    brand: DriverCoreBrand,
}

impl DriverCoreProvider {
    const fn glorious() -> Self {
        Self {
            brand: DriverCoreBrand::Glorious,
        }
    }
    const fn hitscan() -> Self {
        Self {
            brand: DriverCoreBrand::Hitscan,
        }
    }
    const fn gwolves() -> Self {
        Self {
            brand: DriverCoreBrand::GWolves,
        }
    }
    const fn arbiter() -> Self {
        Self {
            brand: DriverCoreBrand::Arbiter,
        }
    }
    const fn lamzu() -> Self {
        Self {
            brand: DriverCoreBrand::Lamzu,
        }
    }
    const fn wlmouse() -> Self {
        Self {
            brand: DriverCoreBrand::WLMouse,
        }
    }

    fn delay_ms(&self) -> u64 {
        match self.brand {
            DriverCoreBrand::Glorious => 50,
            DriverCoreBrand::Hitscan => 200,
            DriverCoreBrand::GWolves | DriverCoreBrand::Arbiter | DriverCoreBrand::Lamzu => 100,
            DriverCoreBrand::WLMouse => 60,
        }
    }

    fn is_cable(&self, endpoint: &Endpoint) -> bool {
        let Some(pid) = endpoint.pid else {
            return false;
        };
        match self.brand {
            DriverCoreBrand::Glorious => GLORIOUS_CABLE_PIDS.contains(&pid),
            DriverCoreBrand::Hitscan => HITSCAN_DRIVERCORE_PIDS.contains(&pid),
            DriverCoreBrand::GWolves => GWOLVES_CABLE_PIDS.contains(&pid),
            DriverCoreBrand::Arbiter => ARBITER_CABLE_PIDS.contains(&pid),
            DriverCoreBrand::Lamzu => {
                (endpoint.vid == Some(0x373E) && pid == 0x001C)
                    || (endpoint.vid == Some(0x37B0) && LAMZU_37B0_CABLE_PIDS.contains(&pid))
            }
            DriverCoreBrand::WLMouse => WLMOUSE_CABLE_PIDS.contains(&pid),
        }
    }
}

impl Provider for DriverCoreProvider {
    fn id(&self) -> &str {
        match self.brand {
            DriverCoreBrand::Glorious => "glorious.mouse",
            DriverCoreBrand::Hitscan => "hitscan.mouse",
            DriverCoreBrand::GWolves => "gwolves.mouse",
            DriverCoreBrand::Arbiter => "arbiter.akitsu",
            DriverCoreBrand::Lamzu => "lamzu.mouse",
            DriverCoreBrand::WLMouse => "wlmouse.mouse",
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let pid = endpoint.pid.unwrap_or(0);
        let name = match self.brand {
            DriverCoreBrand::Glorious => match pid {
                0x2011 | 0x2022 => "Glorious Model O Wireless",
                0x2012 | 0x2023 => "Glorious Model D Wireless",
                0x2013 | 0x2024 => "Glorious Model O- Wireless",
                0x2014 | 0x2025 => "Glorious Model D- Wireless",
                0x2033 => "Glorious Model O 2 Wireless",
                0x201A | 0x2034 | 0x2036 => "Glorious Model D 2 PRO",
                _ => "Glorious wireless mouse",
            },
            DriverCoreBrand::Hitscan => match pid {
                0x0136 => "Hitscan Hyperlight X",
                0x0194 => "Hitscan Hyperlight N",
                _ => "Hitscan Hyperlight",
            },
            DriverCoreBrand::GWolves => "G-Wolves wireless mouse",
            DriverCoreBrand::Arbiter => match pid {
                0x0009 | 0x000A => "Arbiter Studio Akitsu",
                0x0038 | 0x0039 => "Arbiter Studio Akitsu 3950",
                0x0066 | 0x0067 => "Arbiter Studio Akitsu Medium",
                _ => "Arbiter Studio Akitsu",
            },
            DriverCoreBrand::Lamzu => match (endpoint.vid, pid) {
                (Some(0x373E), 0x0016) | (Some(0x37B0), 0x001F) => "LAMZU Maya X V2",
                (Some(0x373E), _) | (Some(0x37B0), 0x003A | 0x003B | 0x0044) => "LAMZU Maya X",
                (Some(0x37B0), 0x0005 | 0x000B | 0x000C) => "LAMZU Tachi",
                (Some(0x37B0), 0x001C | 0x001E) => "LAMZU Tachi Lite",
                (Some(0x37B0), 0x0009 | 0x000F | 0x0010) => "LAMZU Inca",
                (Some(0x37B0), 0x0011 | 0x0013 | 0x0015 | 0x003C | 0x0050) => "LAMZU Maya",
                (Some(0x37B0), 0x0007 | 0x000D | 0x000E) => "LAMZU Paro",
                (Some(0x37B0), 0x0017 | 0x0019 | 0x001B) => "LAMZU Thorn",
                (Some(0x37B0), 0x0021 | 0x0003 | 0x0023 | 0x0030 | 0x0032 | 0x0040) => {
                    "LAMZU Thorn V2"
                }
                (Some(0x37B0), 0x0025 | 0x0027) => "LAMZU Atlantis OG Champion",
                (Some(0x37B0), 0x0028 | 0x002A | 0x002B | 0x003E | 0x003F) => "LAMZU Atlantis Mini",
                (Some(0x37B0), 0x002C | 0x0052) => "LAMZU Maya M",
                (Some(0x37B0), 0x0042 | 0x0056) => "LAMZU Orcus",
                _ => "LAMZU wireless mouse",
            },
            DriverCoreBrand::WLMouse => match pid {
                0xA874 => "WLMouse YING Magnesium",
                0xA856 => "WLMouse Beast X V2",
                0xA860 | 0xA861 => "WLMouse Beast G",
                0xA868 => "WLMouse Beast X Mini Pro",
                0xA870 | 0xA871 => "WLMouse Beast X Pro",
                0xA878 | 0xA879 => "WLMouse Sword X 8K",
                0xA880 => "WLMouse Beast MAX",
                0xA883 | 0xA884 => "WLMouse Beast X 8K",
                0xA885 => "WLMouse Beast X Mini",
                _ => "WLMouse wireless mouse",
            },
        };
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|n| !n.is_empty() && matches!(self.brand, DriverCoreBrand::GWolves))
                .unwrap_or_else(|| name.to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = match self.brand {
            DriverCoreBrand::Glorious => {
                endpoint.vid == Some(0x258A)
                    && endpoint.pid.is_some_and(|p| GLORIOUS_PIDS.contains(&p))
                    && endpoint.usage_page == Some(0xFFFF)
            }
            DriverCoreBrand::Hitscan => {
                endpoint.vid == Some(0x3770)
                    && endpoint
                        .pid
                        .is_some_and(|p| HITSCAN_DRIVERCORE_PIDS.contains(&p))
                    && endpoint.usage_page == Some(0xFFFF)
            }
            DriverCoreBrand::GWolves => {
                endpoint.vid == Some(0x33E4) && endpoint.usage_page == Some(0xFFFF)
            }
            DriverCoreBrand::Arbiter => {
                endpoint.vid == Some(0x373E)
                    && endpoint.pid.is_some_and(|p| ARBITER_PIDS.contains(&p))
                    && endpoint.usage_page == Some(0xFFFF)
            }
            DriverCoreBrand::Lamzu => {
                ((endpoint.vid == Some(0x373E)
                    && endpoint.pid.is_some_and(|p| LAMZU_373E_PIDS.contains(&p)))
                    || endpoint.vid == Some(0x37B0))
                    && endpoint.usage_page == Some(0xFFFF)
            }
            DriverCoreBrand::WLMouse => {
                endpoint.vid == Some(0x36A7)
                    && endpoint.pid.is_some_and(|p| WLMOUSE_PIDS.contains(&p))
                    && endpoint.usage_page == Some(0xFFFF)
            }
        };
        if matched {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        let mut req = vec![0_u8; 65];
        req[3] = 0x02;
        req[4] = 0x02;
        req[6] = 0x83;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        let cable = self.is_cable(device.endpoint);
        for attempt in 0..3 {
            sleep(deadline, Duration::from_millis(self.delay_ms()))?;
            let r = feature_frame(session, device, 0x00, 64)?;
            let parsed = if r.len() >= 9 && r[4] == 0x02 && r[6] == 0x83 {
                Some((r[1], r[7], r[8]))
            } else if r.len() >= 8 && r[3] == 0x02 && r[5] == 0x83 {
                Some((r[0], r[6], r[7]))
            } else {
                None
            };
            if let Some((status, chg, level)) = parsed {
                if status == 0xA0 && attempt < 2 {
                    continue;
                }
                if matches!(status, 0xA0 | 0xA4) {
                    return Ok(ProviderReading {
                        reachability: Reachability::Sleeping,
                        source: ReadingSource::VendorHid,
                        ..ProviderReading::default()
                    });
                }
                if matches!(status, 0xA1 | 0xA2) {
                    if level == 0 && !cable {
                        return Ok(ProviderReading {
                            reachability: Reachability::Offline,
                            source: ReadingSource::VendorHid,
                            ..ProviderReading::default()
                        });
                    }
                    let pct = checked_percentage(level)?;
                    let charge = if cable || chg == 0x01 {
                        ChargeState::Charging
                    } else {
                        ChargeState::Discharging
                    };
                    return Ok(reading(Some(pct), charge, None));
                }
            }
        }
        Err(BatteryError::timeout(self.id()))
    }
}

struct Nordic05Provider {
    sprime: bool,
}

impl Nordic05Provider {
    const fn ninjutso_sora_v2() -> Self {
        Self { sprime: false }
    }
    const fn sprime_pm1() -> Self {
        Self { sprime: true }
    }
}

impl Provider for Nordic05Provider {
    fn id(&self) -> &str {
        if self.sprime {
            "sprime.pm1"
        } else {
            "ninjutso.sora_v2"
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(250))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some(if self.sprime {
            "Sprime PM1".to_owned()
        } else {
            "Ninjutso Sora V2".to_owned()
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let pid_ok = if self.sprime {
            matches!(
                endpoint.pid,
                Some(0xAC11 | 0xAC12 | 0xAC13 | 0xAC1C | 0xAD8C)
            )
        } else {
            matches!(endpoint.pid, Some(0xAE1C | 0xAE11))
        };
        if endpoint.vid == Some(0x1915)
            && pid_ok
            && endpoint.usage_page == Some(0xFFA0)
            && endpoint.usage == Some(0x0001)
        {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        let cable = matches!(device.endpoint.pid, Some(0xAE11 | 0xAC11 | 0xAC12 | 0xAC13));
        let mut req = vec![0_u8; 32];
        req[..5].copy_from_slice(&[0x05, 0x15, 0x00, 0x00, 0x01]);
        for _ in 0..4 {
            send_with_modes(session, device, &[SendMode::Feature], &req)?;
            sleep(deadline, Duration::from_millis(15))?;
            let r = feature_frame(session, device, 0x05, 31)?;
            if r.len() >= 13 && r[0] == 0x05 && r[1] == 0x15 && r[9] <= 100 {
                let connected = cable || r[12] != 0;
                if !connected {
                    return Ok(ProviderReading {
                        reachability: Reachability::Offline,
                        source: ReadingSource::VendorHid,
                        ..ProviderReading::default()
                    });
                }
                if self.sprime {
                    let full = r[11] != 0;
                    let pct = if full { 100 } else { r[9] };
                    let charge = if full {
                        ChargeState::Full
                    } else if r[10] != 0 {
                        ChargeState::Charging
                    } else {
                        ChargeState::Discharging
                    };
                    return Ok(reading(Some(pct), charge, None));
                } else {
                    let pct = if r[9] == 0 { 100 } else { r[9] };
                    let charge = if r[10] != 0 {
                        ChargeState::Charging
                    } else {
                        ChargeState::Discharging
                    };
                    return Ok(reading(Some(pct), charge, None));
                }
            }
            sleep(deadline, Duration::from_millis(90))?;
        }
        Err(BatteryError::timeout(self.id()))
    }
}

struct NinjutsoTenProvider;

impl Provider for NinjutsoTenProvider {
    fn id(&self) -> &str {
        "ninjutso.ten"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(250))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Ninjutso Ten / Ten Air".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x093A)
            && matches!(endpoint.pid, Some(0xEB01 | 0xE020))
            && endpoint.usage_page == Some(0xFF01)
            && endpoint.usage == Some(0x0001)
        {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            let mut req = vec![0_u8; 16];
            req[..5].copy_from_slice(&[0x06, cmd, 0x00, 0x00, 0x01]);
            send_with_modes(session, device, &[SendMode::Feature], &req)?;
            for _ in 0..3 {
                sleep(deadline, Duration::from_millis(30))?;
                let r = feature_frame(session, device, 0x06, 15)?;
                if r.len() >= 9 && r[0] == 0x06 && r[1] == cmd {
                    return Ok(r[8]);
                }
            }
            Err(BatteryError::timeout("Ninjutso Ten"))
        };
        if ask(session, 0x13)? != 1 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = checked_percentage(ask(session, 0x12)?)?;
        let charge = if ask(session, 0x11)? != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct PulsarX2V3Provider;

impl Provider for PulsarX2V3Provider {
    fn id(&self) -> &str {
        "pulsar.x2_v3"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(350))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x3402) => "Pulsar X2 V3".to_owned(),
            Some(0x3406) => "Pulsar X2 V3 eS".to_owned(),
            _ => "Pulsar X2 V3 wireless mouse".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3710)
            && matches!(endpoint.pid, Some(0x3402 | 0x3406 | 0x5402 | 0x5403))
            && endpoint.usage_page.is_some_and(|p| p >= 0xFF00)
        {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        let cable = matches!(device.endpoint.pid, Some(0x3402 | 0x3406));
        let mut req = vec![0_u8; 65];
        req[2] = 0x08;
        req[3] = 0x81;
        req[4] = 0x01;
        let sum: u16 = req[1..63].iter().map(|&b| u16::from(b)).sum();
        req[63] = (sum & 0xFF) as u8;
        req[64] = (sum >> 8) as u8;

        let mut saw_offline = false;
        for _ in 0..3 {
            send_with_modes(session, device, &[SendMode::Feature], &req)?;
            for _ in 0..6 {
                sleep(deadline, Duration::from_millis(50))?;
                let r = feature_frame(session, device, 0x00, 64)?;
                if r.len() >= 65 && r[2] == 0x08 && r[3] == 0x81 && r[4] == 0x01 {
                    match r[1] {
                        0x01 => {
                            let val = r[7];
                            if val == 0xFF {
                                continue;
                            }
                            if val == 0 && !cable {
                                return Ok(ProviderReading {
                                    reachability: Reachability::Offline,
                                    source: ReadingSource::VendorHid,
                                    ..ProviderReading::default()
                                });
                            }
                            let pct = checked_percentage(val)?;
                            let charge = if cable {
                                ChargeState::Charging
                            } else {
                                ChargeState::Unknown
                            };
                            return Ok(reading(Some(pct), charge, None));
                        }
                        0x02 => continue,
                        0x05 => {
                            saw_offline = true;
                            continue;
                        }
                        _ => {}
                    }
                }
            }
        }
        if saw_offline {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        Err(BatteryError::timeout("Pulsar X2 V3"))
    }
}

struct VaxeeProvider;

impl Provider for VaxeeProvider {
    fn id(&self) -> &str {
        "vaxee.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x2002) => "VAXEE 8K Wireless".to_owned(),
            Some(0x1011) => "ZYGEN NP-01S V2 Wireless".to_owned(),
            _ => "VAXEE wireless mouse".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3057)
            && matches!(endpoint.pid, Some(0x1002 | 0x2002 | 0x1011))
            && endpoint.usage_page == Some(0xFF05)
            && endpoint.usage == Some(0x0001)
        {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            let mut req = vec![0_u8; 64];
            req[..5].copy_from_slice(&[0x0E, 0xA5, cmd, 0x01, 0x01]);
            send_with_modes(session, device, &[SendMode::Feature], &req)?;
            sleep(deadline, Duration::from_millis(100))?;
            let r = feature_frame(session, device, 0x0E, 63)?;
            if r.len() >= 6 && r[0] == 0x0E && r[1] == 0xA5 && r[2] == cmd && r[4] == 0x01 {
                Ok(r[5])
            } else {
                Err(BatteryError::invalid_response("invalid VAXEE response"))
            }
        };
        let step = ask(session, 0x0B)?;
        if step > 20 {
            return Err(BatteryError::invalid_response(
                "VAXEE battery step exceeds 20",
            ));
        }
        let charge = match ask(session, 0x10) {
            Ok(0) => ChargeState::Discharging,
            Ok(_) => ChargeState::Charging,
            Err(_) => ChargeState::Unknown,
        };
        Ok(reading(Some(step * 5), charge, None))
    }
}

struct Endgame4kProvider;

impl Provider for Endgame4kProvider {
    fn id(&self) -> &str {
        "endgame.4k"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(450))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Endgame Gear 4K wireless mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3367)
            && endpoint.pid == Some(0x1970)
            && endpoint.usage_page == Some(0xFF01)
            && endpoint.usage == Some(0x0002)
        {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        let mut req = vec![0_u8; 64];
        req[0] = 0xA1;
        req[1] = 0xB4;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        for delay in [150_u64, 250, 350] {
            sleep(deadline, Duration::from_millis(delay))?;
            let r = feature_frame(session, device, 0xA1, 63)?;
            if r.len() >= 20 && r[0] == 0xA1 {
                match r[1] {
                    0x01 => {
                        return Ok(reading(
                            Some(checked_percentage(r[16])?),
                            ChargeState::Unknown,
                            None,
                        ))
                    }
                    0x03 => continue,
                    0x08 | 0x11 => {
                        return Ok(ProviderReading {
                            reachability: Reachability::Sleeping,
                            source: ReadingSource::VendorHid,
                            ..ProviderReading::default()
                        })
                    }
                    _ => {}
                }
            }
        }
        Err(BatteryError::timeout("Endgame Gear 4K"))
    }
}
