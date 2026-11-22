//! Razer wireless headset battery protocols.

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, crc16_poly_1021, drain_input,
    experimental_metadata, feature_frame, input_frame, query, reading, send_with_modes, sleep,
    wait_frame, SendMode, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(RazerHeadsetProvider),
        Arc::new(BlackSharkV2HyperSpeedProvider),
        Arc::new(BarracudaXProvider),
        Arc::new(BlackSharkV2Pro2020Provider),
        Arc::new(BlackSharkV3XProvider),
        Arc::new(BlackSharkV3Provider),
        Arc::new(BarracudaProProvider),
    ]
}

struct RazerHeadsetProvider;

impl Provider for RazerHeadsetProvider {
    fn id(&self) -> &str {
        "razer.headset"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(150),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Razer BlackShark V2 Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1532)
            && endpoint.usage_page == Some(0xFF00)
            && endpoint.pid == Some(0x0555)
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
        let battery = razer_headset_query(session, device, deadline, 0x21)?;
        let percentage = checked_percentage(battery[15])?;
        let charge = match razer_headset_query(session, device, deadline, 0x2A) {
            Ok(frame) => charge_bool(frame[15]),
            Err(BatteryError::Timeout { .. }) => ChargeState::Unknown,
            Err(error) => return Err(error),
        };
        Ok(reading(Some(percentage), charge, None))
    }
}

fn razer_headset_query(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    field: u8,
) -> Result<Vec<u8>, BatteryError> {
    let mut request = vec![0_u8; 64];
    request[0] = 0x02;
    request[1] = 0x80;
    request[2] = 0x09;
    request[5] = 0x50;
    request[6] = 0x41;
    request[7] = 0x08;
    request[9] = 0x04;
    request[10] = field;
    request[12] = 1;
    request[13] = 1;
    query(
        session,
        device,
        deadline,
        request,
        Duration::from_millis(150),
        |frame| {
            Ok(frame.len() >= 16
                && frame[0] == 0x02
                && frame[2..4] == [0x50, 0x49]
                && frame[12] == field
                && frame[13..15] == [1, 1])
        },
    )
}

struct BlackSharkV2HyperSpeedProvider;

impl Provider for BlackSharkV2HyperSpeedProvider {
    fn id(&self) -> &str {
        "razer.blackshark_v2_hs"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Razer BlackShark V2 HyperSpeed".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1532)
            && endpoint.pid == Some(0x0565)
            && matches!(endpoint.usage_page, Some(0xFF14) | Some(0xFF00))
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
        let ask = |session: &mut dyn HidSession, cmd: u8, seq: u8| {
            drain_input(session, device, deadline, 6)?;
            let mut req = vec![0_u8; 64];
            req[0] = 0x02;
            req[2] = seq;
            req[6] = 0x04;
            req[9] = 0x80;
            req[10] = cmd;
            req[62] = req[..62].iter().fold(0_u8, |a, b| a ^ b);
            send_with_modes(session, device, OUTPUT, &req)?;
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |r| {
                    if r.len() < 64 || r[0] != 0x02 || r[2] != seq || r[10] != cmd || r[12] < 1 {
                        return Ok(false);
                    }
                    let xor = r[1..62].iter().fold(0_u8, |a, b| a ^ b);
                    if r[62] != xor {
                        return Err(BatteryError::ChecksumFailed);
                    }
                    Ok(true)
                },
            )
        };
        let bat = ask(session, 0x21, 0x60)?;
        let pct = checked_percentage(bat[13])?;
        let charge = match ask(session, 0x2A, 0x61) {
            Ok(chg) => {
                if chg[13] > 0 {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                }
            }
            Err(BatteryError::Timeout { .. }) => ChargeState::Unknown,
            Err(error) => return Err(error),
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct BarracudaXProvider;

impl Provider for BarracudaXProvider {
    fn id(&self) -> &str {
        "razer.barracuda_x"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(600),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Razer Barracuda X (2022)".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = match endpoint.pid {
            Some(0x0550) => endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(0x0001),
            Some(0x0552) => endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(0x0000),
            _ => false,
        };
        if endpoint.vid == Some(0x1532) && matched {
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
        let macronix = device.endpoint.pid == Some(0x0552);
        drain_input(session, device, deadline, 6)?;
        if macronix {
            let mut init = vec![0_u8; 64];
            init[0] = 0x01;
            init[1] = 0x40;
            send_with_modes(session, device, OUTPUT, &init)?;
            let _ = wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(400),
                8,
                |r| Ok(r.len() >= 4 && r[..4] == [0x01, 0x40, 0x01, 0x01]),
            )?;
        }
        let send_airoha = |session: &mut dyn HidSession, group: u8, seq: u8, body: &[u8]| {
            let mut payload = Vec::with_capacity(4 + body.len());
            payload.extend_from_slice(&[0x50, 0x41, group, seq]);
            payload.extend_from_slice(body);
            let mut pkt = vec![0_u8; 64];
            if macronix {
                pkt[0] = 0x01;
                pkt[1] = 0x80;
                pkt[2] = payload.len() as u8;
                pkt[3..3 + payload.len()].copy_from_slice(&payload);
                send_with_modes(session, device, OUTPUT, &pkt)
            } else {
                pkt[0] = 0x02;
                pkt[1] = 0x80;
                pkt[2] = payload.len() as u8;
                pkt[5..5 + payload.len()].copy_from_slice(&payload);
                let crc = crc16_poly_1021(&pkt, 0x0000);
                pkt[3] = (crc & 0xFF) as u8;
                pkt[4] = (crc >> 8) as u8;
                send_with_modes(session, device, OUTPUT, &pkt)?;
                let status = input_frame(session, device, 0x02, 63)?;
                if status.len() >= 2 && status[1] == b'O' {
                    Ok(())
                } else {
                    Err(BatteryError::DeviceBusy {
                        endpoint: device.endpoint.id.to_string(),
                    })
                }
            }
        };
        send_airoha(session, 0x0E, 1, &[0x02, 0xE1, 0x01])?;
        let _ack = wait_airoha_pi_frame(session, device, deadline, macronix, |f| {
            f.len() >= 13 && f[10] == 0x0E && f[11] == 0x81 && f[12] == 0x00
        })?;
        send_airoha(session, 0x08, 2, &[0x03, 0x21, 0x00, 0x00])?;
        let bat = wait_airoha_pi_frame(session, device, deadline, macronix, |f| {
            f.len() >= 14 && f[2] == 0x08 && f[10] == 0x21 && f[11] == 0x01 && f[12] >= 1
        })?;
        let pct = checked_percentage(bat[13])?;
        let charge = match send_airoha(session, 0x08, 3, &[0x03, 0x2A, 0x00, 0x00]).and_then(|()| {
            wait_airoha_pi_frame(session, device, deadline, macronix, |f| {
                f.len() >= 14 && f[2] == 0x08 && f[10] == 0x2A && f[11] == 0x01 && f[12] >= 1
            })
        }) {
            Ok(f) => charge_bool(f[13]),
            Err(BatteryError::Timeout { .. }) => ChargeState::Unknown,
            Err(error) => return Err(error),
        };
        let _ = send_airoha(session, 0x0E, 4, &[0x02, 0xE1, 0x00]);
        Ok(reading(Some(pct), charge, None))
    }
}

fn wait_airoha_pi_frame(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    macronix: bool,
    pred: impl Fn(&[u8]) -> bool,
) -> Result<Vec<u8>, BatteryError> {
    let mut stream = Vec::new();
    for _ in 0..16 {
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            8,
            |r| {
                if macronix {
                    Ok(r.len() >= 4 && r[0] == 0x01 && r[1] == 0x80)
                } else {
                    Ok(r.len() >= 3 && r[0] == 0x02)
                }
            },
        )?;
        let chunk = if macronix {
            let n = usize::from(frame[2]).min(frame.len().saturating_sub(3));
            &frame[3..3 + n]
        } else {
            let n = usize::from(frame[1]).min(frame.len().saturating_sub(2));
            &frame[2..2 + n]
        };
        stream.extend_from_slice(chunk);
        while let Some(pos) = stream.windows(2).position(|w| w == [0x50, 0x49]) {
            if pos > 0 {
                stream.drain(..pos);
            }
            if stream.len() < 10 {
                break;
            }
            let total = 10 + usize::from(u16::from_le_bytes([stream[8], stream[9]]));
            if stream.len() < total {
                break;
            }
            let msg: Vec<u8> = stream.drain(..total).collect();
            if pred(&msg) {
                return Ok(msg);
            }
        }
    }
    Err(BatteryError::timeout("Razer Barracuda X"))
}

struct BlackSharkV2Pro2020Provider;

impl Provider for BlackSharkV2Pro2020Provider {
    fn id(&self) -> &str {
        "razer.blackshark_v2_pro_2020"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, true),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Razer BlackShark V2 Pro (2020)".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1532)
            && endpoint.pid == Some(0x0528)
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
        let mut req = vec![0_u8; 64];
        req[..9].copy_from_slice(&[0xFF, 0x0A, 0x00, 0xFD, 0x04, 0x12, 0xF1, 0x02, 0x05]);
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        for attempt in 0..5 {
            sleep(deadline, Duration::from_millis(100))?;
            let r = feature_frame(session, device, 0xFF, 63)?;
            if r.len() >= 15 && r[..9] == [0xFF, 0x0F, 0x05, 0xFE, 0x12, 0x04, 0x1F, 0x08, 0x05] {
                let mv = u32::from(u16::from_be_bytes([r[12], r[13]]));
                if mv == 0 {
                    return Ok(ProviderReading {
                        reachability: Reachability::Offline,
                        source: ReadingSource::VendorHid,
                        ..ProviderReading::default()
                    });
                }
                let charging = r[11] == 0x09;
                let pct = if charging {
                    None
                } else {
                    Some(checked_percentage(r[14])?)
                };
                return Ok(reading(
                    pct,
                    if charging {
                        ChargeState::Charging
                    } else {
                        ChargeState::Discharging
                    },
                    Some(mv),
                ));
            }
            if attempt == 4
                && r.len() >= 9
                && r[..9] == [0xFF, 0x01, 0x00, 0xFE, 0x12, 0x04, 0x1F, 0x08, 0x05]
            {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
        }
        Err(BatteryError::timeout("Razer BlackShark V2 Pro (2020)"))
    }
}

struct BlackSharkV3XProvider;

impl Provider for BlackSharkV3XProvider {
    fn id(&self) -> &str {
        "razer.blackshark_v3_x"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Razer BlackShark V3 X".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1532)
            && matches!(endpoint.pid, Some(0x057C | 0x057D))
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
        let ask = |session: &mut dyn HidSession, cmd: u8, txid: u8| {
            let mut req = vec![0_u8; 64];
            req[0] = 0x07;
            req[2] = txid;
            req[6] = 0x02;
            req[7] = 0x07;
            req[8] = cmd;
            req[62] = req[3..=60].iter().fold(0_u8, |a, b| a ^ b);
            send_with_modes(session, device, &[SendMode::Feature], &req)?;
            for _ in 0..4 {
                sleep(deadline, Duration::from_millis(60))?;
                let r = feature_frame(session, device, 0x07, 63)?;
                if r.len() >= 63 && r[2] == txid && r[7] == 0x07 && r[8] == cmd && r[6] >= 2 {
                    match r[1] {
                        0x02 => return Ok(r[10]),
                        0x01 => continue,
                        _ => {
                            return Err(BatteryError::DeviceDisconnected {
                                endpoint: device.endpoint.id.to_string(),
                            })
                        }
                    }
                }
            }
            Err(BatteryError::timeout("Razer BlackShark V3 X"))
        };
        let pct = checked_percentage(ask(session, 0x80, 0x41)?)?;
        let charge = match ask(session, 0x84, 0x42) {
            Ok(v) => charge_bool(v),
            Err(BatteryError::Timeout { .. }) => ChargeState::Unknown,
            Err(error) => return Err(error),
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct BlackSharkV3Provider;

impl Provider for BlackSharkV3Provider {
    fn id(&self) -> &str {
        "razer.blackshark_v3"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(600),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0576 | 0x0577) => "Razer BlackShark V3 Pro".to_owned(),
            Some(0x0A4E | 0x0A55) => "Razer BlackShark V3 Pro for Xbox".to_owned(),
            _ => "Razer BlackShark V3".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1532)
            && matches!(
                endpoint.pid,
                Some(0x0576 | 0x0577 | 0x0579 | 0x057A | 0x0A4E | 0x0A55)
            )
            && endpoint.interface_number == Some(5)
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
        let packet = |dir: u8, cls: u8| {
            let mut b = vec![0_u8; 64];
            b[0] = 0x02;
            b[2] = 0x60;
            b[6] = 0x04;
            b[9] = dir;
            b[10] = cls;
            b[62] = b[..62].iter().fold(0_u8, |a, x| a ^ x);
            b
        };
        send_with_modes(session, device, OUTPUT, &packet(0x00, 0x02))?;
        send_with_modes(session, device, OUTPUT, &packet(0x00, 0x2A))?;
        send_with_modes(session, device, OUTPUT, &packet(0x80, 0x2A))?;
        sleep(deadline, Duration::from_millis(200))?;
        drain_input(session, device, deadline, 6)?;

        send_with_modes(session, device, OUTPUT, &packet(0x80, 0x21))?;
        let bat = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| Ok(r.len() >= 15 && r[0] == 0x02 && r[2] == 0x60 && r[10] == 0x21),
        )?;
        let pct = checked_percentage(bat[13])?;
        let mut charging = bat[14] != 0;
        if !charging {
            send_with_modes(session, device, OUTPUT, &packet(0x80, 0x2A))?;
            if let Ok(chg) = wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(300),
                8,
                |r| Ok(r.len() >= 15 && r[0] == 0x02 && r[2] == 0x60 && r[10] == 0x2A),
            ) {
                charging = chg[13] == 1;
            }
        }
        Ok(reading(
            Some(pct),
            if charging {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            },
            None,
        ))
    }
}

struct BarracudaProProvider;

impl Provider for BarracudaProProvider {
    fn id(&self) -> &str {
        "razer.barracuda_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Razer Barracuda Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1532)
            && endpoint.pid == Some(0x053A)
            && endpoint.usage_page == Some(0xFF00)
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
        let remote = |on: bool| {
            let mut b = vec![0_u8; 64];
            b[..10].copy_from_slice(&[
                0x01,
                0x80,
                0x07,
                b'P',
                b'A',
                0x0E,
                0x08,
                0x02,
                0xE1,
                u8::from(on),
            ]);
            b
        };
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            let mut b = vec![0_u8; 64];
            b[..9].copy_from_slice(&[0x01, 0x80, 0x08, b'P', b'A', 0x08, 0x08, 0x03, cmd]);
            send_with_modes(session, device, OUTPUT, &b)?;
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(400),
                8,
                |r| {
                    Ok(r.len() >= 17
                        && r[13] == cmd
                        && matches!(r[14], 0x01 | 0x02)
                        && r[15] > 0
                        && r[15] <= 32)
                },
            )
        };
        drain_input(session, device, deadline, 6)?;
        send_with_modes(session, device, OUTPUT, &remote(true))?;
        sleep(deadline, Duration::from_millis(50))?;
        let bat = ask(session, 0x21);
        let result = match bat {
            Ok(b) => {
                let pct = checked_percentage(b[16])?;
                let charge = match ask(session, 0x2A) {
                    Ok(c) => {
                        if c[16] > 0 {
                            ChargeState::Charging
                        } else {
                            ChargeState::Discharging
                        }
                    }
                    Err(BatteryError::Timeout { .. }) => ChargeState::Unknown,
                    Err(error) => return Err(error),
                };
                Ok(reading(Some(pct), charge, None))
            }
            Err(error) => Err(error),
        };
        let _ = send_with_modes(session, device, OUTPUT, &remote(false));
        result
    }
}
