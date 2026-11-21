//! HyperX wireless headset and mouse battery protocols (VIDs 0x03F0, 0x0951).

use crate::error::BatteryError;
use crate::model::{ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource};
use crate::providers::fixed::{
    capabilities, checked_percentage, drain_input, experimental_metadata, feature_frame,
    interpolate_mv, reading, send_with_modes, sleep, wait_frame, SendMode, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

const VID_HP: u16 = 0x03F0;
const VID_KINGSTON: u16 = 0x0951;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(Cloud3Provider),
        Arc::new(Cloud3SProvider),
        Arc::new(Cloud2Provider),
        Arc::new(Cloud2KingstonProvider),
        Arc::new(CloudAlphaProvider),
        Arc::new(CloudAlpha2AndHasteProvider),
        Arc::new(CloudFlightProvider),
        Arc::new(StingerCoreProvider),
    ]
}

fn hyperx_query(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    req: &[u8],
    timeout: Duration,
    validate: impl Fn(&[u8]) -> Result<bool, BatteryError>,
) -> Result<Vec<u8>, BatteryError> {
    drain_input(session, device, deadline, 6)?;
    send_with_modes(session, device, OUTPUT, req)?;
    wait_frame(session, device, deadline, timeout, 16, validate)
}

struct Cloud3Provider;

impl Provider for Cloud3Provider {
    fn id(&self) -> &str {
        "hyperx.cloud3"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(450))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x069F | 0x0995) => "HyperX Cloud II Core Wireless".to_owned(),
            _ => "HyperX Cloud III Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_HP)
            && matches!(endpoint.pid, Some(0x05B7 | 0x0C9D | 0x069F | 0x0995))
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
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            let mut req = vec![0_u8; 62];
            req[0] = 0x66;
            req[1] = cmd;
            hyperx_query(
                session,
                device,
                deadline,
                &req,
                Duration::from_millis(450),
                |r| Ok(r.len() >= 5 && r[0] == 0x66 && r[1] == cmd),
            )
        };
        let conn = ask(session, 0x82)?;
        if conn[2] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let chg = ask(session, 0x8A)?;
        let bat = ask(session, 0x89)?;
        let pct = checked_percentage(bat[4])?;
        let mv = u32::from(u16::from_be_bytes([bat[2], bat[3]]));
        let charge = if chg[2] == 1 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, (mv > 0).then_some(mv)))
    }
}

struct Cloud3SProvider;

impl Provider for Cloud3SProvider {
    fn id(&self) -> &str {
        "hyperx.cloud3_s"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("HyperX Cloud III S Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_HP)
            && matches!(endpoint.pid, Some(0x06BE | 0x02CC))
            && matches!(endpoint.usage_page, Some(0x01C0) | Some(0xFF00..=0xFFFF))
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
            req[..6].copy_from_slice(&[0x0C, 0x02, 0x03, 0x01, 0x00, cmd]);
            hyperx_query(
                session,
                device,
                deadline,
                &req,
                Duration::from_millis(500),
                |r| Ok(r.len() >= 7 && r[0] == 0x0C && r[5] == cmd && r[6] != 0xFF),
            )
        };
        let conn = ask(session, 0x02)?;
        if conn[6] != 2 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let bat = ask(session, 0x06)?;
        let chg = ask(session, 0x48)?;
        let pct = checked_percentage(bat[6])?;
        let charge = match chg[6] {
            1 | 2 => ChargeState::Charging,
            0 => ChargeState::Discharging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct Cloud2Provider;

impl Provider for Cloud2Provider {
    fn id(&self) -> &str {
        "hyperx.cloud2"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("HyperX Cloud II Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_HP)
            && matches!(endpoint.pid, Some(0x0696 | 0x018B | 0x0D93 | 0x0C8C))
            && endpoint.usage_page == Some(0xFF90)
            && endpoint.usage == Some(0x0303)
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
            req[..4].copy_from_slice(&[0x06, 0xFF, 0xBB, cmd]);
            hyperx_query(
                session,
                device,
                deadline,
                &req,
                Duration::from_millis(500),
                |r| Ok(r.len() >= 8 && r[..4] == [0x06, 0xFF, 0xBB, cmd]),
            )
        };
        let bat = ask(session, 0x02)?;
        let chg = ask(session, 0x03)?;
        let pct = checked_percentage(bat[7])?;
        let mv = u32::from(u16::from_be_bytes([bat[5], bat[6]]));
        let charge = if chg[4] == 1 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, (mv > 0).then_some(mv)))
    }
}

struct Cloud2KingstonProvider;

impl Provider for Cloud2KingstonProvider {
    fn id(&self) -> &str {
        "hyperx.cloud2_kingston"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x16EA | 0x16EB) => "HyperX Cloud Flight S".to_owned(),
            _ => "HyperX Cloud II Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(VID_KINGSTON) {
            return MatchDecision::NoMatch;
        }
        let matched = match endpoint.pid {
            Some(0x1718) => endpoint.usage_page.is_some_and(|p| p >= 0xFF00),
            Some(0x16EA | 0x16EB) => {
                endpoint.usage_page == Some(0xFF13) && endpoint.usage == Some(0x0001)
            }
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
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            let mut req = vec![0_u8; 62];
            req[..17].copy_from_slice(&[
                0x06, 0x00, 0x02, 0x00, 0x9A, 0x00, 0x00, 0x68, 0x4A, 0x8E, 0x0A, 0x00, 0x00, 0x00,
                0xBB, cmd, 0x00,
            ]);
            hyperx_query(
                session,
                device,
                deadline,
                &req,
                Duration::from_millis(500),
                |r| Ok(r.len() >= 8 && r[0] == 0x0B && r[2] == 0xBB && r[3] == cmd),
            )
        };
        let bat = ask(session, 0x02)?;
        let chg = ask(session, 0x03)?;
        let pct = checked_percentage(bat[7])?;
        let charge = if chg[4] == 1 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct CloudAlphaProvider;

impl Provider for CloudAlphaProvider {
    fn id(&self) -> &str {
        "hyperx.cloud_alpha"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("HyperX Cloud Alpha Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_HP)
            && endpoint.pid == Some(0x098D)
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
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            let mut req = vec![0_u8; 32];
            req[..3].copy_from_slice(&[0x21, 0xBB, cmd]);
            hyperx_query(
                session,
                device,
                deadline,
                &req,
                Duration::from_millis(500),
                |r| Ok(r.len() >= 4 && r[0] == 0x21 && r[2] == cmd),
            )
        };
        let conn = ask(session, 0x03)?;
        if conn[3] == 0x01 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let chg = ask(session, 0x0C)?;
        let bat = ask(session, 0x0B)?;
        let pct = checked_percentage(bat[3])?;
        let charge = if chg[3] == 0x01 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct CloudAlpha2AndHasteProvider;

impl Provider for CloudAlpha2AndHasteProvider {
    fn id(&self) -> &str {
        "hyperx.alpha2_haste"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x08BE) => "HyperX Cloud Alpha 2 Wireless".to_owned(),
            Some(0x0AB5) => "HyperX Pulsefire Haste 2 Core Wireless".to_owned(),
            Some(0x0DA0) => "HyperX Pulsefire Haste 2 Mini".to_owned(),
            _ => "HyperX Pulsefire Haste 2 Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(VID_HP) {
            return MatchDecision::NoMatch;
        }
        let matched = match endpoint.pid {
            Some(0x08BE) => endpoint.usage_page == Some(0xFF13) && endpoint.usage == Some(0xFF00),
            Some(0x0AB5 | 0x0DA0 | 0x0D97 | 0x0F98) => {
                endpoint.usage_page.is_some_and(|p| p >= 0xFF00)
            }
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
        let mut link_req = vec![0_u8; 32];
        link_req[..2].copy_from_slice(&[0x52, 0x01]);
        let link = hyperx_query(
            session,
            device,
            deadline,
            &link_req,
            Duration::from_millis(500),
            |r| Ok(r.len() >= 8 && r[0] == 0x53 && r[1] == 0x01),
        )?;
        if link[2] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let mut bat_req = vec![0_u8; 32];
        bat_req[..2].copy_from_slice(&[0x50, 0x02]);
        let bat = hyperx_query(
            session,
            device,
            deadline,
            &bat_req,
            Duration::from_millis(500),
            |r| Ok(r.len() >= 8 && r[0] == 0x51 && r[1] == 0x02),
        )?;
        let pct = checked_percentage(bat[2])?;
        let mv = u32::from(u16::from_le_bytes([bat[6], bat[7]]));
        let is_mouse = device.endpoint.pid != Some(0x08BE);
        let charge = match bat[3] {
            0x01 => ChargeState::Charging,
            0x02 if is_mouse => ChargeState::Charging,
            0x00 => ChargeState::Discharging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, (mv > 0).then_some(mv)))
    }
}

struct CloudFlightProvider;

impl Provider for CloudFlightProvider {
    fn id(&self) -> &str {
        "hyperx.cloud_flight"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("HyperX Cloud Flight".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let vid_pid = (endpoint.vid == Some(VID_KINGSTON)
            && matches!(endpoint.pid, Some(0x16C4 | 0x1723 | 0x1749)))
            || (endpoint.vid == Some(VID_HP) && endpoint.pid == Some(0x0E90));
        if vid_pid && endpoint.usage_page.is_some_and(|p| p >= 0xFF00) {
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
        let mut req = vec![0_u8; 20];
        req[..3].copy_from_slice(&[0x21, 0xFF, 0x05]);
        let r = hyperx_query(
            session,
            device,
            deadline,
            &req,
            Duration::from_millis(500),
            |f| Ok(f.len() >= 5),
        )?;
        let mv = u32::from(u16::from_be_bytes([r[3], r[4]]));
        if mv > 0x100B {
            return Ok(reading(None, ChargeState::Charging, Some(mv)));
        }
        let pct = if mv <= 3648 {
            ((f64::from(mv) * 0.00125).round() as i64).clamp(0, 100) as u8
        } else if mv > 3975 {
            100
        } else {
            let v = f64::from(mv);
            let poly = 2.547_505e-8 * v.powi(4) - 3.900_299e-4 * v.powi(3) + 2.238_321 * v.powi(2)
                - 5_706.256 * v
                + 5_452_299.0;
            (poly.round() as i64).clamp(0, 100) as u8
        };
        Ok(reading(Some(pct), ChargeState::Discharging, Some(mv)))
    }
}

const STINGER_DISCHARGING_TABLE: &[(u32, u8)] = &[
    (4121, 100),
    (4042, 90),
    (3954, 80),
    (3905, 70),
    (3838, 60),
    (3785, 50),
    (3763, 40),
    (3738, 30),
    (3700, 20),
    (3651, 10),
    (3517, 0),
];

const STINGER_CHARGING_TABLE: &[(u32, u8)] = &[
    (4130, 100),
    (4090, 90),
    (4050, 80),
    (4000, 70),
    (3967, 60),
    (3946, 50),
    (3930, 40),
    (3916, 30),
    (3871, 20),
    (3831, 10),
    (3400, 0),
];

struct StingerCoreProvider;

impl Provider for StingerCoreProvider {
    fn id(&self) -> &str {
        "hyperx.stinger_core"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(300))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0A8E) => "HyperX Cloud Stinger Core DTS".to_owned(),
            Some(0x173F) => "HyperX Cloud Core Wireless DTS".to_owned(),
            _ => "HyperX Cloud Stinger Core Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let vid_pid = (endpoint.vid == Some(VID_HP)
            && matches!(endpoint.pid, Some(0x0A93 | 0x0A8E)))
            || (endpoint.vid == Some(VID_KINGSTON)
                && matches!(endpoint.pid, Some(0x170B | 0x1713 | 0x173F)));
        if vid_pid && endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(0x0001) {
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
        let poll_loop = matches!(device.endpoint.pid, Some(0x0A93 | 0x1713 | 0x173F | 0x0A8E));
        let mut conn_req = vec![0_u8; 32];
        conn_req[..20].copy_from_slice(&[
            0xFF, 0x07, 0x00, 0xFD, 0x04, 0x00, 0x00, 0x02, 0x82, 0x03, 0x05, 0x29, 0x0F, 0x00,
            0x00, 0x03, 0x68, 0x10, 0x00, 0x00,
        ]);
        send_with_modes(session, device, &[SendMode::Feature], &conn_req)?;
        let mut conn = None;
        for _ in 0..if poll_loop { 10 } else { 1 } {
            sleep(deadline, Duration::from_millis(20))?;
            let f = feature_frame(session, device, 0xFF, 31)?;
            if f.len() >= 13 && !(poll_loop && f[1] == 0x01 && f[2] == 0x00) {
                conn = Some(f);
                break;
            }
        }
        let conn = conn.ok_or_else(|| BatteryError::timeout("HyperX Stinger Core connect"))?;
        if conn[11] != 0x01 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }

        let mut bat_req = vec![0_u8; 32];
        bat_req[..12].copy_from_slice(&[
            0xFF, 0x07, 0x00, 0xFD, 0x04, 0x0C, 0xF1, 0x02, 0x01, 0x04, 0xF0, 0x0C,
        ]);
        send_with_modes(session, device, &[SendMode::Feature], &bat_req)?;
        let mut bat = None;
        for _ in 0..if poll_loop { 10 } else { 1 } {
            sleep(deadline, Duration::from_millis(20))?;
            let f = feature_frame(session, device, 0xFF, 31)?;
            if f.len() >= 13 && (!poll_loop || f[..3] == [0xFF, 0x12, 0x05]) {
                bat = Some(f);
                break;
            }
        }
        let bat = bat.ok_or_else(|| BatteryError::timeout("HyperX Stinger Core battery"))?;
        let mv = u32::from(u16::from_le_bytes([bat[11], bat[12]]));
        let (pct, charge) = match bat[9] {
            0x06 => (100, ChargeState::Full),
            0x05 => (
                interpolate_mv(mv, STINGER_CHARGING_TABLE),
                ChargeState::Charging,
            ),
            0x03 => (
                interpolate_mv(mv, STINGER_DISCHARGING_TABLE),
                ChargeState::Discharging,
            ),
            _ => (
                interpolate_mv(mv, STINGER_DISCHARGING_TABLE),
                ChargeState::Unknown,
            ),
        };
        Ok(reading(Some(pct), charge, (mv > 0).then_some(mv)))
    }
}
