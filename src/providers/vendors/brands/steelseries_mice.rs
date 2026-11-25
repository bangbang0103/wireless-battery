//! SteelSeries wireless mouse battery protocols (VID 0x1038).

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    capabilities, checked_percentage, drain_input, experimental_metadata, reading, send_with_modes,
    unnumbered_payload, wait_frame, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

const VID_STEELSERIES: u16 = 0x1038;
const AEROX_WIRED_PIDS: &[u16] = &[
    0x183A, 0x187A, 0x1892, 0x1854, 0x185A, 0x1876, 0x1842, 0x184A, 0x185E, 0x1862,
];
const AEROX_RX_PIDS: &[u16] = &[
    0x1838, 0x1878, 0x1890, 0x1852, 0x1858, 0x1874, 0x1840, 0x1848, 0x185C, 0x1860,
];

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(AeroxProvider),
        Arc::new(RivalProvider),
        Arc::new(RivalProProvider),
    ]
}

struct AeroxProvider;

impl Provider for AeroxProvider {
    fn id(&self) -> &str {
        "steelseries.aerox"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let name = match endpoint.pid {
            Some(0x1838 | 0x183A) => "SteelSeries Aerox 3 Wireless",
            Some(0x1878 | 0x187A) => "SteelSeries Aerox 3 Wireless CS2",
            Some(0x1890 | 0x1892) => "SteelSeries Aerox 3 Wireless Gen 2",
            Some(0x1852 | 0x1854) => "SteelSeries Aerox 5 Wireless",
            Some(0x185C | 0x185E) => "SteelSeries Aerox 5 Wireless Destiny 2",
            Some(0x1860 | 0x1862) => "SteelSeries Aerox 5 Wireless Diablo IV",
            Some(0x1858 | 0x185A) => "SteelSeries Aerox 9 Wireless",
            Some(0x1874 | 0x1876) => "SteelSeries Aerox 9 Wireless WoW",
            Some(0x1840 | 0x1842) => "SteelSeries Prime Wireless",
            Some(0x1848 | 0x184A) => "SteelSeries Prime Mini Wireless",
            _ => "SteelSeries Wireless Mouse",
        };
        Some(name.to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let pid_ok = endpoint
            .pid
            .is_some_and(|p| AEROX_WIRED_PIDS.contains(&p) || AEROX_RX_PIDS.contains(&p));
        if endpoint.vid == Some(VID_STEELSERIES)
            && pid_ok
            && endpoint.interface_number == Some(3)
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
        let pid = device.endpoint.pid.unwrap_or(0);
        let wired = AEROX_WIRED_PIDS.contains(&pid);
        let percent_mode = matches!(pid, 0x1890 | 0x1892);
        let cmd = if wired { 0x92 } else { 0xD2 };
        drain_input(session, device, deadline, 8)?;
        let mut req = vec![0_u8; 64];
        req[1] = cmd;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(300),
            16,
            |f| {
                let r = unnumbered_payload(f);
                if r.len() >= 2 && r[0] == 0x40 && r[1] == 0xFF {
                    return Ok(true);
                }
                let min_len = if percent_mode { 3 } else { 2 };
                Ok(r.len() >= min_len && r[0] == cmd)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r.len() >= 2 && r[0] == 0x40 && r[1] == 0xFF {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        if percent_mode {
            if r[1] > 0x02 {
                return Err(BatteryError::invalid_response(
                    "invalid SteelSeries Gen 2 status byte",
                ));
            }
            if !wired && r[1] == 0 && r[2] == 0 {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            let charge = match r[1] {
                0x00 => ChargeState::Discharging,
                0x01 => ChargeState::Charging,
                0x02 => ChargeState::Full,
                _ => ChargeState::Unknown,
            };
            let pct = if r[2] == 0xFF {
                None
            } else {
                Some(checked_percentage(r[2])?)
            };
            Ok(reading(pct, charge, None))
        } else {
            let step = r[1] & 0x7F;
            if step == 0 {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            if step > 21 {
                return Err(BatteryError::invalid_response(
                    "invalid SteelSeries step value",
                ));
            }
            let pct = ((step - 1) * 5).min(100);
            let charge = if (r[1] & 0x80) != 0 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            Ok(reading(Some(pct), charge, None))
        }
    }
}

struct RivalProvider;

impl Provider for RivalProvider {
    fn id(&self) -> &str {
        "steelseries.rival"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x1830) => "SteelSeries Rival 3 Wireless".to_owned(),
            Some(0x1872) => "SteelSeries Rival 3 Wireless Gen 2".to_owned(),
            _ => "SteelSeries Rival 650 Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(VID_STEELSERIES)
            || !endpoint.usage_page.is_some_and(|p| p >= 0xFF00)
        {
            return MatchDecision::NoMatch;
        }
        let matched = match endpoint.pid {
            Some(0x1830 | 0x1872) => endpoint.interface_number == Some(3),
            Some(0x1726 | 0x172B) => endpoint.interface_number == Some(0),
            _ => false,
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
        drain_input(session, device, deadline, 8)?;
        let mut req = vec![0_u8; 64];
        req[1] = 0xAA;
        req[2] = 0x01;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(300),
            8,
            |f| Ok(f[0] == 0x00 && unnumbered_payload(f).len() >= 3),
        )?;
        let r = unnumbered_payload(&frame);
        let pct = checked_percentage(r[0])?;
        let charge = if r[2] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct RivalProProvider;

impl Provider for RivalProProvider {
    fn id(&self) -> &str {
        "steelseries.rival_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("SteelSeries Rival Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_STEELSERIES)
            && endpoint.pid == Some(0x1880)
            && endpoint.interface_number == Some(2)
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
        drain_input(session, device, deadline, 8)?;
        let mut req = vec![0_u8; 64];
        req[1] = 0x92;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            8,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 3 && r[0] == 0x92 && r[1] == 0x00)
            },
        )?;
        let r = unnumbered_payload(&frame);
        Ok(reading(
            Some(checked_percentage(r[2])?),
            ChargeState::Unknown,
            None,
        ))
    }
}
