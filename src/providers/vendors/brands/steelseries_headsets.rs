//! SteelSeries wireless headset battery protocols (VID 0x1038).

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

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(Arctis1Provider),
        Arc::new(Arctis7Provider),
        Arc::new(Arctis9Provider),
        Arc::new(ArctisProWirelessProvider),
        Arc::new(NovaProWirelessProvider),
        Arc::new(NovaFamilyProvider),
        Arc::new(NovaEliteProvider),
    ]
}

fn checked_map_to_100(value: u8, lo: u8, hi: u8) -> Result<u8, BatteryError> {
    if hi <= lo || value < lo || value > hi {
        return Err(BatteryError::invalid_response(format!(
            "SteelSeries battery value {value} out of expected range {lo}..={hi}"
        )));
    }
    checked_percentage(((u16::from(value - lo) * 100) / u16::from(hi - lo)) as u8)
}

struct Arctis1Provider;

impl Provider for Arctis1Provider {
    fn id(&self) -> &str {
        "steelseries.arctis1"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("SteelSeries Arctis 1 Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_STEELSERIES)
            && matches!(endpoint.pid, Some(0x12B3 | 0x12B6 | 0x12D5 | 0x12D7))
            && endpoint.usage_page == Some(0xFF43)
            && endpoint.usage == Some(0x0202)
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
        let mut req = vec![0_u8; 32];
        req[..2].copy_from_slice(&[0x06, 0x12]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| Ok(f.len() >= 4 && f[0] == 0x06 && f[1] == 0x12),
        )?;
        if frame[2] == 0x01 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        Ok(reading(
            Some(checked_percentage(frame[3])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct Arctis7Provider;

impl Provider for Arctis7Provider {
    fn id(&self) -> &str {
        "steelseries.arctis7"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("SteelSeries Arctis 7 Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_STEELSERIES)
            && matches!(endpoint.pid, Some(0x1260 | 0x12AD | 0x1252))
            && endpoint.usage_page == Some(0xFF43)
            && endpoint.usage == Some(0x0202)
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
        let mut req = vec![0_u8; 32];
        req[..2].copy_from_slice(&[0x06, 0x18]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| Ok(f.len() >= 3 && f[0] == 0x06 && f[1] == 0x18),
        )?;
        if frame[2] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        Ok(reading(
            Some(checked_percentage(frame[2])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct Arctis9Provider;

impl Provider for Arctis9Provider {
    fn id(&self) -> &str {
        "steelseries.arctis9"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("SteelSeries Arctis 9".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_STEELSERIES)
            && matches!(endpoint.pid, Some(0x12C2 | 0x12C4))
            && endpoint.interface_number == Some(0)
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
        let mut req = vec![0_u8; 32];
        req[1] = 0x20;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| {
                let p = unnumbered_payload(f);
                Ok(p.len() >= 5 && p[0] == 0x20)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let pct = checked_map_to_100(r[3], 0x64, 0x9A)?;
        let charge = match r[4] {
            0x01 => ChargeState::Charging,
            0x00 => ChargeState::Discharging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct ArctisProWirelessProvider;

impl Provider for ArctisProWirelessProvider {
    fn id(&self) -> &str {
        "steelseries.arctis_pro_wireless"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("SteelSeries Arctis Pro Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_STEELSERIES)
            && endpoint.pid == Some(0x1290)
            && endpoint.interface_number == Some(0)
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
        let mut link_req = vec![0_u8; 32];
        link_req[1] = 0x41;
        link_req[2] = 0xAA;
        send_with_modes(session, device, OUTPUT, &link_req)?;
        let link = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| Ok(f[0] == 0x00 && !unnumbered_payload(f).is_empty()),
        )?;
        if unnumbered_payload(&link)[0] == 0x02 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let mut bat_req = vec![0_u8; 32];
        bat_req[1] = 0x40;
        bat_req[2] = 0xAA;
        send_with_modes(session, device, OUTPUT, &bat_req)?;
        let bat = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| Ok(f[0] == 0x00 && !unnumbered_payload(f).is_empty()),
        )?;
        let raw = unnumbered_payload(&bat)[0];
        if raw > 4 {
            return Err(BatteryError::invalid_response(
                "Arctis Pro Wireless level exceeds 4",
            ));
        }
        Ok(reading(
            Some(checked_map_to_100(raw, 0, 4)?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct NovaProWirelessProvider;

impl Provider for NovaProWirelessProvider {
    fn id(&self) -> &str {
        "steelseries.nova_pro_wireless"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("SteelSeries Arctis Nova Pro Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_STEELSERIES)
            && matches!(endpoint.pid, Some(0x12E0 | 0x12E5 | 0x225D))
            && endpoint.interface_number == Some(4)
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
        req[..2].copy_from_slice(&[0x06, 0xB0]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| Ok(f.len() >= 16 && f[0] == 0x06 && f[1] == 0xB0),
        )?;
        if frame[15] == 0x01 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let charge = match frame[15] {
            0x02 => ChargeState::Charging,
            0x00 | 0x08 => ChargeState::Discharging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(
            Some(checked_map_to_100(frame[6], 0, 8)?),
            charge,
            None,
        ))
    }
}

const NOVA_7_PLUS_PIDS: &[u16] = &[0x220E, 0x2212, 0x2216, 0x2236];
const NOVA_3P_5_PIDS: &[u16] = &[0x2269, 0x226D, 0x2232, 0x2253];
const NOVA_7_DISCRETE_PIDS: &[u16] = &[0x2202, 0x2206, 0x220A, 0x223A, 0x227A, 0x22A4];
const NOVA_7_PCT_PIDS: &[u16] = &[
    0x22A1, 0x227E, 0x2258, 0x2298, 0x229E, 0x22AD, 0x22A9, 0x22A5, 0x22A7,
];

struct NovaFamilyProvider;

impl Provider for NovaFamilyProvider {
    fn id(&self) -> &str {
        "steelseries.nova_family"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let name = match endpoint.pid {
            Some(p) if NOVA_7_PLUS_PIDS.contains(&p) => "SteelSeries Arctis 7+",
            Some(0x2269 | 0x226D) => "SteelSeries Arctis Nova 3P Wireless",
            Some(0x2232 | 0x2253) => "SteelSeries Arctis Nova 5",
            Some(0x230A) => "SteelSeries Arctis GameBuds",
            _ => "SteelSeries Arctis Nova 7",
        };
        Some(name.to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let pid_ok = endpoint.pid.is_some_and(|p| {
            p == 0x230A
                || NOVA_7_PLUS_PIDS.contains(&p)
                || NOVA_3P_5_PIDS.contains(&p)
                || NOVA_7_DISCRETE_PIDS.contains(&p)
                || NOVA_7_PCT_PIDS.contains(&p)
        });
        if endpoint.vid == Some(VID_STEELSERIES)
            && pid_ok
            && endpoint.usage_page == Some(0xFFC0)
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
        drain_input(session, device, deadline, 8)?;
        let mut req = vec![0_u8; 64];
        req[1] = 0xB0;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| {
                let p = unnumbered_payload(f);
                Ok(p.len() >= 7 && p[0] == 0xB0)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let pid = device.endpoint.pid.unwrap_or(0);
        if NOVA_7_PLUS_PIDS.contains(&pid) {
            if r[1] == 0x01 {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            let charge = if r[3] == 0x01 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            Ok(reading(Some(checked_map_to_100(r[2], 0, 4)?), charge, None))
        } else if NOVA_3P_5_PIDS.contains(&pid) {
            if r[1] == 0x02 {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            let charge = if r[4] == 0x01 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            Ok(reading(Some(checked_percentage(r[3])?), charge, None))
        } else if pid == 0x230A {
            let left = if r[3] == 0x03 {
                Some(checked_percentage(r[5])?)
            } else {
                None
            };
            let right = if r[4] == 0x03 {
                Some(checked_percentage(r[6])?)
            } else {
                None
            };
            let pct = match (left, right) {
                (Some(l), Some(rt)) => Some(l.min(rt)),
                (Some(l), None) => Some(l),
                (None, Some(rt)) => Some(rt),
                (None, None) => None,
            };
            if let Some(pct) = pct {
                Ok(reading(Some(pct), ChargeState::Unknown, None))
            } else {
                Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                })
            }
        } else {
            if r[3] == 0x00 {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            let charge = match r[3] {
                0x01 | 0x02 => ChargeState::Charging,
                0x03 => ChargeState::Discharging,
                _ => ChargeState::Unknown,
            };
            let pct = if NOVA_7_DISCRETE_PIDS.contains(&pid) {
                checked_map_to_100(r[2], 0, 4)?
            } else {
                checked_percentage(r[2])?
            };
            Ok(reading(Some(pct), charge, None))
        }
    }
}

struct NovaEliteProvider;

impl Provider for NovaEliteProvider {
    fn id(&self) -> &str {
        "steelseries.nova_elite"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("SteelSeries Arctis Nova Elite".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_STEELSERIES)
            && endpoint.pid == Some(0x2244)
            && endpoint.interface_number == Some(3)
            && endpoint.usage_page == Some(0xFFC0)
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
        req[..2].copy_from_slice(&[0x01, 0xB0]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |r| {
                Ok((r.len() > 15 && r[0] == 0x01 && r[1] == 0xB0)
                    || (r.len() >= 5 && r[0] == 0x07 && r[1] == 0xB7))
            },
        )?;
        if frame[0] == 0x01 && frame[1] == 0xB0 {
            if frame[14] == 0x01 {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            let pct = checked_percentage(frame[6])?;
            let charging = frame[15] == 0x02 || frame[14] == 0x02;
            Ok(reading(
                Some(pct),
                if charging {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                },
                None,
            ))
        } else {
            let pct = checked_percentage(frame[2])?;
            let charge = if frame[4] == 0x02 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            Ok(reading(Some(pct), charge, None))
        }
    }
}
