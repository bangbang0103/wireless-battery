//! Wireless esports gaming mouse battery protocols:
//! Nordic 0x05/0x15 family (Ninjutso Sora V2, Sprime PM1), Ninjutso Ten,
//! Pulsar X2 V3, VAXEE 4K/8K, and Endgame Gear 4K.

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
        Arc::new(Nordic05Provider::ninjutso_sora_v2()),
        Arc::new(Nordic05Provider::sprime_pm1()),
        Arc::new(NinjutsoTenProvider),
        Arc::new(PulsarX2V3Provider),
        Arc::new(VaxeeProvider),
        Arc::new(Endgame4kProvider),
    ]
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(250),
        )
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
                if self.sprime {
                    if !connected {
                        return Ok(ProviderReading {
                            reachability: Reachability::Offline,
                            source: ReadingSource::VendorHid,
                            ..ProviderReading::default()
                        });
                    }
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
                    if !connected {
                        return Ok(ProviderReading {
                            reachability: Reachability::Sleeping,
                            source: ReadingSource::VendorHid,
                            ..ProviderReading::default()
                        });
                    }
                    let full = r[11] != 0;
                    if r[9] == 0 && !full {
                        return Ok(ProviderReading {
                            reachability: Reachability::Sleeping,
                            source: ReadingSource::VendorHid,
                            ..ProviderReading::default()
                        });
                    }
                    let pct = if full { 100 } else { r[9] };
                    let charge = if full {
                        ChargeState::Full
                    } else if r[10] != 0 {
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(250),
        )
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(350),
        )
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(450),
        )
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
