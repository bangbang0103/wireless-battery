//! DriverCore 0x83 wireless gaming mouse battery protocol family
//! (Glorious, Hitscan, G-Wolves, Arbiter Studio, LAMZU, WLMouse).

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
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
    0xA883, 0xA884, 0xA885, 0xA887,
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
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
                0xA887 => "WLMouse Beast X",
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
                    let charge = match chg {
                        0x01 => ChargeState::Charging,
                        0x02 => ChargeState::Full,
                        _ if cable => ChargeState::Charging,
                        0x00 => ChargeState::Discharging,
                        _ => ChargeState::Unknown,
                    };
                    return Ok(reading(Some(pct), charge, None));
                }
            }
        }
        Err(BatteryError::timeout(self.id()))
    }
}
