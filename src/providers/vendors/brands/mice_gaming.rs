//! Gaming mouse HID battery protocols:
//! Cooler Master, NZXT, MSI, Fantech XD5, Raigor, Sinowealth, Royal Kludge,
//! Darmoshark, Machenike, Niceboy, and AULA SC580.

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, checksum_sum, drain_input,
    experimental_metadata, feature_frame, reading, send_with_modes, sleep, unnumbered_payload,
    wait_frame, SendMode, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(CoolerMasterProvider),
        Arc::new(NzxtLiftEliteProvider),
        Arc::new(MsiQueryProvider),
        Arc::new(MsiGm41Provider),
        Arc::new(FantechXd5Provider),
        Arc::new(RaigorProvider),
        Arc::new(SinowealthReceiverProvider),
        Arc::new(RoyalKludgeProvider),
        Arc::new(DarmosharkM3Provider),
        Arc::new(DarmosharkNrfProvider),
        Arc::new(MachenikeL7Provider),
        Arc::new(NiceboyMw400Provider),
        Arc::new(AulaSc580Provider),
    ]
}

struct CoolerMasterProvider;

impl Provider for CoolerMasterProvider {
    fn id(&self) -> &str {
        "coolermaster.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0169) => "Cooler Master MM712".to_owned(),
            Some(0x0167) => "Cooler Master MM731".to_owned(),
            _ => "Cooler Master wireless mouse".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x2516)
            && matches!(endpoint.pid, Some(0x0167 | 0x0169 | 0x01B7))
            && endpoint.usage_page == Some(0xFF0A)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 32];
        req[1] = 0x46;
        req[2] = 0x81;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 13 && r[0] == 0x47 && r[1] == 0x81)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[10] == 0 || r[10] > 100 {
            return Err(BatteryError::invalid_response(
                "invalid Cooler Master battery percentage",
            ));
        }
        let charge = if r[11] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(r[10]), charge, None))
    }
}

struct NzxtLiftEliteProvider;

impl Provider for NzxtLiftEliteProvider {
    fn id(&self) -> &str {
        "nzxt.lift_elite"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, true),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("NZXT Lift Elite Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1E71)
            && matches!(endpoint.pid, Some(0x2101 | 0x2129))
            && endpoint.usage_page == Some(0xFFCA)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 65];
        req[1..6].copy_from_slice(&[0x4E, 0x02, 0x81, 0x00, 0xB0]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            10,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 27 && r[0] == 0x4E && r[1] == 0x02 && r[2] == 0x97)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let pct_u16 = u16::from_le_bytes([r[17], r[18]]);
        if pct_u16 > 100 {
            return Err(BatteryError::invalid_response(
                "NZXT battery percentage exceeds 100",
            ));
        }
        let mv = u32::from(u16::from_be_bytes([r[8], r[9]]));
        let charge = if (r[22] & 0x01) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(
            Some(pct_u16 as u8),
            charge,
            (3000..=4500).contains(&mv).then_some(mv),
        ))
    }
}

struct MsiQueryProvider;

impl Provider for MsiQueryProvider {
    fn id(&self) -> &str {
        "msi.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x9E22) => "MSI VERSA 300 ELITE WIRELESS".to_owned(),
            _ => "MSI Clutch GM31 Lightweight Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(0x0DB0) {
            return MatchDecision::NoMatch;
        }
        let matched = match endpoint.pid {
            Some(0x9E22) => endpoint.usage_page == Some(0xFF0D) && endpoint.usage == Some(0x00B0),
            Some(0x0D3C | 0x0D3B) => {
                endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(0x0001)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 65];
        req[1] = 0x0F;
        req[2] = 0x20;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 15 && r[0] == 0x0F && r[1] == 0x20)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let level = r[14] & 0x7F;
        let charging = (r[14] & 0x80) != 0;
        if level == 0 && !charging {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        Ok(reading(
            Some(checked_percentage(level)?),
            if charging {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            },
            None,
        ))
    }
}

struct MsiGm41Provider;

impl Provider for MsiGm41Provider {
    fn id(&self) -> &str {
        "msi.gm41"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(1000),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("MSI Clutch GM41 Lightweight Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0DB0)
            && matches!(endpoint.pid, Some(0x0D4B | 0x0D4C))
            && endpoint.usage_page == Some(0xFF10)
            && endpoint.usage == Some(0x0006)
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
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(1000),
            16,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 7 && r[0] == 0xA0 && r[2] == 0x07)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let raw_pct = (u32::from(r[1]) * 51 + 1760 + 50) / 100;
        if raw_pct > 105 {
            return Err(BatteryError::invalid_response(
                "invalid MSI GM41 battery ADC byte",
            ));
        }
        Ok(reading(
            Some(raw_pct.min(100) as u8),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct FantechXd5Provider;

impl Provider for FantechXd5Provider {
    fn id(&self) -> &str {
        "fantech.xd5"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0xFA7E) => "Redragon Invader Pro".to_owned(),
            _ => "Fantech Helios Go XD5".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x25A7)
            && matches!(endpoint.pid, Some(0xFA7B | 0xFA7C | 0xFA7E))
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
        let mut req = vec![0_u8; 17];
        req[0] = 0x08;
        req[1] = 0x04;
        req[16] = 0x49;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        let r = match wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| Ok(f.len() >= 8 && (f[0] == 0x04 || f[1] == 0x04)),
        ) {
            Ok(f) => f,
            Err(BatteryError::Timeout { .. }) => feature_frame(session, device, 0x08, 16)?,
            Err(error) => return Err(error),
        };
        let (power, chg) = if r.len() >= 8 && r[1] == 0x04 {
            (r[6], r[7])
        } else if r.len() >= 7 && r[0] == 0x04 {
            (r[5], r[6])
        } else {
            return Err(BatteryError::invalid_response(
                "invalid Fantech XD5 battery response",
            ));
        };
        let pct = if device.endpoint.pid == Some(0xFA7E) {
            checked_percentage(power.saturating_mul(10))?
        } else {
            checked_percentage(power)?
        };
        Ok(reading(Some(pct), charge_bool(chg), None))
    }
}

struct RaigorProvider;

impl Provider for RaigorProvider {
    fn id(&self) -> &str {
        "raigor.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Fantech Raigor / 2.4 GHz mouse".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1EA7)
            && matches!(endpoint.pid, Some(0x0064 | 0x0066))
            && endpoint.usage_page == Some(0xFFB5)
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
        drain_input(session, device, deadline, 6)?;
        let req = [0xB5, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| {
                Ok(r.len() == 8
                    && r[0] == 0xB5
                    && ((r[1] == 0x05 && r[2] == 0x94 && r[3] == 0x04) || r[1] == 0x67))
            },
        )?;
        let raw = if r[1] == 0x67 { r[7] } else { r[6] };
        Ok(reading(
            Some(checked_percentage(raw)?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct SinowealthReceiverProvider;

impl Provider for SinowealthReceiverProvider {
    fn id(&self) -> &str {
        "sinowealth.receiver"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Redragon M602-KS / Sinowealth mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x258A)
            && endpoint.pid == Some(0x002F)
            && endpoint.usage_page == Some(0xFF00)
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
            let req = [0x05, cmd, 0, 0, 0, 0, 0, 0];
            send_with_modes(session, device, &[SendMode::Feature], &req)?;
            sleep(deadline, Duration::from_millis(100))?;
            let r = feature_frame(session, device, 0x05, 7)?;
            if r.len() >= 4 && r[0] == 0x05 && r[1] == cmd {
                Ok(r)
            } else {
                Err(BatteryError::invalid_response(
                    "invalid Sinowealth receiver response",
                ))
            }
        };
        let link = ask(session, 0x80)?;
        if link[2] != 0x01 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let bat = ask(session, 0x90)?;
        if bat[2] != 0x11 {
            return Err(BatteryError::invalid_response(
                "unexpected Sinowealth battery tag",
            ));
        }
        Ok(reading(
            Some(checked_percentage(bat[3])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct RoyalKludgeProvider;

impl Provider for RoyalKludgeProvider {
    fn id(&self) -> &str {
        "royal_kludge.mg6_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, true),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Royal Kludge MG6 Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = match endpoint.pid {
            Some(0x00A6 | 0x00E6) => {
                endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(0x0001)
            }
            Some(0x00A5 | 0x00E5) => {
                endpoint.usage_page == Some(0xFFF2) && endpoint.usage == Some(0x0001)
            }
            _ => false,
        };
        if endpoint.vid == Some(0x2E3C) && matched {
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
        let cable = matches!(device.endpoint.pid, Some(0x00A5 | 0x00E5));
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 65];
        req[1..5].copy_from_slice(&[0x01, 0x01, 0x02, 0x01]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 11 && (r[0] & 0x1F) == 0x01 && (r[1] & 0x0F) == 0x01 && r[2] == 0x02)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if ((r[3] >> 4) & 1) == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct_u16 = u16::from_le_bytes([r[6], r[7]]);
        if pct_u16 > 100 {
            return Err(BatteryError::invalid_response(
                "Royal Kludge battery percentage exceeds 100",
            ));
        }
        let mv = u32::from(u16::from_le_bytes([r[4], r[5]]));
        let charge = if cable || r[10] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(
            Some(pct_u16 as u8),
            charge,
            (3000..=4500).contains(&mv).then_some(mv),
        ))
    }
}

struct DarmosharkM3Provider;

impl Provider for DarmosharkM3Provider {
    fn id(&self) -> &str {
        "darmoshark.m3"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0xFF31) => "Darmoshark M3S".to_owned(),
            _ => "Darmoshark M3".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x248A)
            && matches!(endpoint.pid, Some(0xFF30 | 0xFF31))
            && endpoint.usage_page == Some(0x008C)
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
        let mouse_pid: u16 = if device.endpoint.pid == Some(0xFF31) {
            0xFF18
        } else {
            0xFF12
        };
        let mut req = vec![0_u8; 21];
        req[0] = 0x51;
        req[1] = 0x06;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        for _ in 0..8 {
            sleep(deadline, Duration::from_millis(30))?;
            let r = feature_frame(session, device, 0x51, 20)?;
            if r.len() >= 12
                && r[0] == 0x51
                && r[1] == 0x06
                && r[2] == 0x01
                && u16::from_le_bytes([r[4], r[5]]) == 0x248A
                && u16::from_le_bytes([r[6], r[7]]) == mouse_pid
            {
                return Ok(reading(
                    Some(checked_percentage(r[11])?),
                    ChargeState::Unknown,
                    None,
                ));
            }
        }
        Err(BatteryError::timeout("Darmoshark M3"))
    }
}

struct DarmosharkNrfProvider;

impl Provider for DarmosharkNrfProvider {
    fn id(&self) -> &str {
        "darmoshark.nrf"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Darmoshark M3S Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1915)
            && endpoint.pid == Some(0x0723)
            && endpoint.usage_page == Some(0xFF0A)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 65];
        req[1] = 0x41;
        req[3] = 0x81;
        req[4] = 0x01;
        req[64] = 0xA1_u8.wrapping_sub(checksum_sum(&req[1..64]));
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 64 && r[0] == 0x41 && r[2] == 0x8C && r[3] == 0x01)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[63] != 0xA1_u8.wrapping_sub(checksum_sum(&r[..63])) {
            return Err(BatteryError::ChecksumFailed);
        }
        Ok(reading(
            Some(checked_percentage(r[6])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct MachenikeL7Provider;

impl Provider for MachenikeL7Provider {
    fn id(&self) -> &str {
        "machenike.l7"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(250),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Machenike L7 Mix".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x248A)
            && matches!(endpoint.pid, Some(0xFF4C | 0xFF4E))
            && endpoint.usage_page == Some(0x008C)
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
        let cable = device.endpoint.pid == Some(0xFF4E);
        let mut req = vec![0_u8; 21];
        req[0] = 0x51;
        req[1] = 0x06;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        for _ in 0..10 {
            sleep(deadline, Duration::from_millis(10))?;
            let r = feature_frame(session, device, 0x51, 20)?;
            if r.len() >= 13
                && r[0] == 0x51
                && r[1] == 0x06
                && u16::from_le_bytes([r[4], r[5]]) == 0x248A
                && u16::from_le_bytes([r[6], r[7]]) == 0xFF4E
                && (r[10] & 0x07) == u8::from(!cable)
            {
                let connected = cable || (r[10] & 0x08) != 0;
                if !connected {
                    return Ok(ProviderReading {
                        reachability: Reachability::Offline,
                        source: ReadingSource::VendorHid,
                        ..ProviderReading::default()
                    });
                }
                return Ok(reading(
                    Some(checked_percentage(r[11])?),
                    charge_bool(r[12]),
                    None,
                ));
            }
        }
        Err(BatteryError::timeout("Machenike L7"))
    }
}

struct NiceboyMw400Provider;

impl Provider for NiceboyMw400Provider {
    fn id(&self) -> &str {
        "niceboy.mw400"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, true),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Niceboy MW400".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x248A)
            && endpoint.pid == Some(0xFA02)
            && endpoint.usage_page == Some(0x0001)
            && endpoint.usage == Some(0x0000)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 33];
        req[..4].copy_from_slice(&[0x05, 0x24, 0xFA, 0x48]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| Ok(r.len() >= 7 && r[0] == 0x05 && r[2] == 0xFA && r[3] == 0x48),
        )?;
        if (r[4] | r[5] | r[6]) == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let mv = (u32::from(u16::from_le_bytes([r[4], r[5]])) * 990) / 240;
        let pct = match mv {
            4200.. => 100,
            4100..=4199 => 90,
            4000..=4099 => 80,
            3900..=3999 => 70,
            3800..=3899 => 60,
            3700..=3799 => 50,
            3500..=3699 => 40,
            3400..=3499 => 30,
            3200..=3399 => 20,
            _ => 10,
        };
        let charge = if r[6] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, Some(mv)))
    }
}

struct AulaSc580Provider;

impl Provider for AulaSc580Provider {
    fn id(&self) -> &str {
        "aula.sc580"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("AULA SC580".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let vid_pid = (endpoint.vid == Some(0x088D) && endpoint.pid == Some(0x062E))
            || (endpoint.vid == Some(0x089D) && endpoint.pid == Some(0x062F));
        if vid_pid && endpoint.usage_page == Some(0xFF02) && endpoint.usage == Some(0x0000) {
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 33];
        req[1] = 0x30;
        req[3] = 0x01;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 32 && r[0] == 0x30 && matches!(r[1], 0x00 | 0x02))
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[1] == 0x02 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        if r[31] != checksum_sum(&r[4..31]) {
            return Err(BatteryError::ChecksumFailed);
        }
        if r[14] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        if r[12] == 1 {
            // SC580 firmware freezes percentage at 100 while charging; keep percentage unknown.
            return Ok(reading(None, ChargeState::Charging, None));
        }
        let pct = checked_percentage(r[13])?;
        Ok(reading(Some(pct), ChargeState::Discharging, None))
    }
}
