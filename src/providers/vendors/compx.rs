//! Shared CompX wireless mouse protocol (VXE R1, ATK A9 Air+, IMECOO P1).

use crate::error::BatteryError;
use crate::model::{Endpoint, ProviderReading, Reachability, ReadingSource, SupportStatus};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, checksum_sum, drain_input,
    experimental_metadata, reading, send_with_modes, wait_frame, FixedProvider, FixedSpec, OUTPUT,
    OUTPUT_THEN_SET, SET_OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

static VXE_R1: FixedSpec = FixedSpec {
    id: "vxe.r1",
    name: "VXE R1 Star Flash",
    status: SupportStatus::Experimental,
    matches: match_vxe,
    query: compx_query,
    modes: SET_OUTPUT,
    attempts: 3,
    max_reports: 12,
    io_timeout: Duration::from_millis(800),
    capabilities: capabilities(true, true, true),
    parse: parse_compx,
};

static ATK_A9: FixedSpec = FixedSpec {
    id: "atk.a9_air",
    name: "ATK A9 Air+",
    status: SupportStatus::Experimental,
    matches: match_atk_a9,
    query: compx_query,
    modes: SET_OUTPUT,
    attempts: 3,
    max_reports: 12,
    io_timeout: Duration::from_millis(700),
    capabilities: capabilities(true, true, true),
    parse: parse_compx,
};

static IMECOO_P1: FixedSpec = FixedSpec {
    id: "imecoo.p1",
    name: "IMECOO P1",
    status: SupportStatus::Experimental,
    matches: match_imecoo,
    query: compx_query,
    modes: OUTPUT_THEN_SET,
    attempts: 3,
    max_reports: 12,
    io_timeout: Duration::from_millis(700),
    capabilities: capabilities(true, true, true),
    parse: parse_compx,
};

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(FixedProvider::new(&VXE_R1)),
        Arc::new(FixedProvider::new(&ATK_A9)),
        Arc::new(FixedProvider::new(&IMECOO_P1)),
        Arc::new(Compx08Provider::kysona()),
        Arc::new(Compx08Provider::chimera()),
        Arc::new(Compx08Provider::cherry_xtrfy()),
        Arc::new(Compx08Provider::corsair()),
    ]
}

fn match_vxe(endpoint: &Endpoint) -> bool {
    endpoint.vid == Some(0x373B)
        && endpoint.pid == Some(0x10C9)
        && endpoint.usage_page == Some(0xFF02)
}

fn match_atk_a9(endpoint: &Endpoint) -> bool {
    endpoint.vid == Some(0x373B)
        && endpoint.pid == Some(0x1278)
        && endpoint.usage_page == Some(0xFF02)
}

fn match_imecoo(endpoint: &Endpoint) -> bool {
    endpoint.vid == Some(0x3554)
        && endpoint.pid == Some(0xFB34)
        && endpoint.usage_page == Some(0xFF02)
}

fn compx_query() -> Vec<u8> {
    let mut frame = vec![0_u8; 17];
    frame[0] = 0x08;
    frame[1] = 0x04;
    frame[16] = 0x49;
    frame
}

fn parse_compx(frame: &[u8]) -> Result<Option<ProviderReading>, BatteryError> {
    if frame.len() < 2 || frame[0] != 0x08 || frame[1] != 0x04 {
        return Ok(None);
    }
    if frame.len() != 17 {
        return Err(BatteryError::invalid_response("invalid CompX frame length"));
    }
    if frame[1..]
        .iter()
        .fold(0_u8, |sum, byte| sum.wrapping_add(*byte))
        != 0x4D
    {
        return Err(BatteryError::ChecksumFailed);
    }
    if frame[2..6].iter().all(|byte| *byte == 0) && frame[8] == 0 && frame[9] == 0 {
        return Ok(None);
    }
    if frame[2] != 0 {
        return Err(BatteryError::invalid_response("CompX command was rejected"));
    }
    let voltage = u32::from(u16::from_be_bytes([frame[8], frame[9]]));
    Ok(Some(reading(
        Some(checked_percentage(frame[6])?),
        charge_bool(frame[7]),
        Some(voltage),
    )))
}

#[derive(Clone, Copy)]
enum Compx08Kind {
    Kysona,
    Chimera,
    CherryXtrfy,
    Corsair,
}

struct Compx08Provider {
    kind: Compx08Kind,
}

impl Compx08Provider {
    const fn kysona() -> Self {
        Self {
            kind: Compx08Kind::Kysona,
        }
    }
    const fn chimera() -> Self {
        Self {
            kind: Compx08Kind::Chimera,
        }
    }
    const fn cherry_xtrfy() -> Self {
        Self {
            kind: Compx08Kind::CherryXtrfy,
        }
    }
    const fn corsair() -> Self {
        Self {
            kind: Compx08Kind::Corsair,
        }
    }
}

impl Provider for Compx08Provider {
    fn id(&self) -> &str {
        match self.kind {
            Compx08Kind::Kysona => "kysona.mouse",
            Compx08Kind::Chimera => "chimera.v2",
            Compx08Kind::CherryXtrfy => "cherry.xtrfy",
            Compx08Kind::Corsair => "corsair.compx",
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let name = match self.kind {
            Compx08Kind::Chimera => "Chimera V2",
            Compx08Kind::CherryXtrfy => "Cherry XTRFY wireless mouse",
            Compx08Kind::Corsair => match endpoint.pid {
                Some(0x2B34) => "Corsair Sabre v2 PRO WIRELESS MG",
                _ => "Corsair Sabre V2 Pro Ultralight",
            },
            Compx08Kind::Kysona => match (endpoint.vid, endpoint.pid) {
                (Some(0x3554), Some(0xF55D | 0xF55E)) => "Redragon M916 Pro",
                (Some(0x3554), Some(0xF5D5)) => "Taidu TSG808",
                (Some(0x3554), Some(0xF58A | 0xF58C | 0xF58E | 0xF58F | 0xF505 | 0xF506)) => {
                    "VXE Dragonfly R1"
                }
                (Some(0x35BB), _) => "HECATE wireless mouse",
                (Some(0x3710), _) => "Pulsar wireless mouse",
                (Some(0x3770), _) => "Hitscan Hyperlight",
                (Some(0x373B), _) => "ATK / VXE wireless mouse",
                (Some(0x33E4), _) => "G-Wolves wireless mouse",
                _ => "Kysona / CompX wireless mouse",
            },
        };
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|n| !n.is_empty() && matches!(self.kind, Compx08Kind::Kysona))
                .unwrap_or_else(|| name.to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.usage_page != Some(0xFF02) || endpoint.usage != Some(0x0002) {
            return MatchDecision::NoMatch;
        }
        let matched = match self.kind {
            Compx08Kind::Chimera => endpoint.vid == Some(0x3554) && endpoint.pid == Some(0xF512),
            Compx08Kind::CherryXtrfy => {
                endpoint.vid == Some(0x046A) && endpoint.pid == Some(0x0330)
            }
            Compx08Kind::Corsair => {
                endpoint.vid == Some(0x1B1C) && matches!(endpoint.pid, Some(0x2B2A | 0x2B34))
            }
            Compx08Kind::Kysona => match (endpoint.vid, endpoint.pid) {
                (Some(0x3554), Some(pid)) => !matches!(pid, 0xFA09 | 0xFB40 | 0xFB34 | 0xF512),
                (Some(0x373B), Some(pid)) => !matches!(
                    pid,
                    0x10C9
                        | 0x1278
                        | 0x1262
                        | 0x1012
                        | 0x1154
                        | 0x124F
                        | 0x1155
                        | 0x1216
                        | 0x1178
                        | 0x11CA
                        | 0x11CB
                        | 0x1203
                        | 0x1204
                ),
                (Some(0x3770), Some(pid)) => !matches!(pid, 0x0136 | 0x0194),
                (Some(0x33E4), _) => true,
                (Some(0x3710), Some(pid)) => !matches!(pid, 0x3402 | 0x3406 | 0x5402 | 0x5403),
                (Some(0x35BB), Some(0xD3FA..=0xD3FD)) => true,
                _ => false,
            },
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
            drain_input(session, device, deadline, 6)?;
            let mut req = vec![0_u8; 17];
            req[0] = 0x08;
            req[1] = cmd;
            req[16] = 0x55_u8.wrapping_sub(checksum_sum(&req[..16]));
            send_with_modes(session, device, OUTPUT, &req)?;
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |r| {
                    if r.len() < 17 || r[0] != 0x08 || r[1] != cmd {
                        return Ok(false);
                    }
                    if r[16] != 0x55_u8.wrapping_sub(checksum_sum(&r[..16])) {
                        return Err(BatteryError::ChecksumFailed);
                    }
                    Ok(true)
                },
            )
        };
        let is_chimera = matches!(self.kind, Compx08Kind::Chimera);
        if let Ok(link) = ask(session, 0x03) {
            let online = if is_chimera {
                link[5] != 0
            } else {
                link[6] != 0
            };
            if !online {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
        }
        let bat = ask(session, 0x04)?;
        if is_chimera && bat[5] != 0x02 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = checked_percentage(bat[6])?;
        let mv = u32::from(u16::from_be_bytes([bat[8], bat[9]]));
        Ok(reading(
            Some(pct),
            charge_bool(bat[7]),
            (mv > 0).then_some(mv),
        ))
    }
}
