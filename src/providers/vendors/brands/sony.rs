//! Sony DualSense, DualShock 4, INZONE, and PULSE 3D headset battery protocols (VID 0x054C).

use crate::error::BatteryError;
use crate::model::{ChargeState, DeviceKind, Endpoint, HidBusType, ProviderReading, Reachability};
use crate::providers::fixed::{
    capabilities, checked_percentage, checksum_sum, drain_input, experimental_metadata, frame,
    reading, send_with_modes, wait_frame, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::{HidSession, TransportError};
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(DualSenseProvider),
        Arc::new(DualShock4Provider),
        Arc::new(SonyInzoneProvider),
        Arc::new(SonyPulse3dProvider),
    ]
}

struct DualSenseProvider;

impl Provider for DualSenseProvider {
    fn id(&self) -> &str {
        "sony.dualsense"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Gamepad,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0DF2) => "Sony DualSense Edge".to_owned(),
            _ => "Sony DualSense".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x054C)
            && matches!(endpoint.pid, Some(0x0CE6) | Some(0x0DF2))
            && endpoint.usage_page == Some(1)
            && endpoint.usage == Some(5)
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
        let bluetooth = is_bluetooth_endpoint(device.endpoint);
        if bluetooth {
            // Reading calibration feature report 0x05 switches a Bluetooth DualSense
            // into full 0x31 input report mode without writing calibration data.
            let _ = session
                .get_feature(0x05, 40)
                .map_err(|error| error.into_battery(&device.endpoint.id))?;
        }
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(300),
            8,
            |frame| Ok(matches!(frame.first(), Some(0x01) | Some(0x31))),
        )?;
        let status_index = match frame[0] {
            0x01 if frame.len() >= 54 => 53,
            0x31 if frame.len() >= 55 => 54,
            0x01 | 0x31 => {
                return Err(BatteryError::invalid_response("truncated DualSense report"))
            }
            _ => return Err(BatteryError::invalid_response("unknown DualSense report")),
        };
        let status = frame[status_index];
        let mut percentage = (status & 0x0F).saturating_mul(10).min(100);
        let charge = match status >> 4 {
            0 => ChargeState::Discharging,
            1 => ChargeState::Charging,
            2 => {
                percentage = 100;
                ChargeState::Full
            }
            _ => ChargeState::Unknown,
        };
        Ok(ProviderReading {
            reachability: Reachability::Online,
            ..reading(Some(percentage), charge, None)
        })
    }
}

struct DualShock4Provider;

impl Provider for DualShock4Provider {
    fn id(&self) -> &str {
        "sony.dualshock4"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Gamepad,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0BA0) => "Sony DualShock 4 (wireless adapter)".to_owned(),
            _ => "Sony DualShock 4".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x054C)
            && matches!(endpoint.pid, Some(0x05C4) | Some(0x09CC) | Some(0x0BA0))
            && endpoint.usage_page == Some(1)
            && endpoint.usage == Some(5)
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
        let bluetooth = is_bluetooth_endpoint(device.endpoint);
        if bluetooth {
            let _ = session
                .get_feature(0x05, 40)
                .map_err(|error| error.into_battery(&device.endpoint.id))?;
        }
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(300),
            8,
            |frame| {
                Ok(if bluetooth {
                    frame.first() == Some(&0x11)
                } else {
                    matches!(frame.first(), Some(0x01) | Some(0x11))
                })
            },
        )?;
        let status_index = match frame[0] {
            0x01 if frame.len() >= 31 => 30,
            0x11 if frame.len() >= 33 => 32,
            0x01 | 0x11 => {
                return Err(BatteryError::invalid_response(
                    "truncated DualShock 4 report",
                ))
            }
            _ => return Err(BatteryError::invalid_response("unknown DualShock 4 report")),
        };
        let status = frame[status_index];
        if device.endpoint.pid == Some(0x0BA0) && status == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                ..reading(None, ChargeState::Unknown, None)
            });
        }
        let level = status & 0x0F;
        let cable = (status & 0x10) != 0;
        let (percentage, charge) = if cable {
            if level >= 11 {
                (100, ChargeState::Full)
            } else {
                ((level.saturating_mul(10)).min(100), ChargeState::Charging)
            }
        } else {
            (
                (level.saturating_mul(10).saturating_add(5)).min(100),
                ChargeState::Discharging,
            )
        };
        Ok(ProviderReading {
            reachability: Reachability::Online,
            ..reading(Some(percentage), charge, None)
        })
    }
}

fn is_bluetooth_endpoint(endpoint: &Endpoint) -> bool {
    endpoint.bus_type == Some(HidBusType::Bluetooth)
        || endpoint.system_path.to_ascii_lowercase().contains("bth")
        || endpoint
            .system_path
            .to_ascii_lowercase()
            .contains("bluetooth")
}

struct SonyInzoneProvider;

impl Provider for SonyInzoneProvider {
    fn id(&self) -> &str {
        "sony.inzone"
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
            Some(0x0FA8) => "Sony INZONE H9 II".to_owned(),
            Some(0x0EBF) => "Sony INZONE H5".to_owned(),
            Some(0x0EC2) => "Sony INZONE Buds".to_owned(),
            _ => "Sony INZONE headset".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x054C)
            && matches!(endpoint.pid, Some(0x0FA8 | 0x0EBF | 0x0EC2))
            && endpoint.usage_page == Some(0xFF04)
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
        let buds = device.endpoint.pid == Some(0x0EC2);
        let min_payload = if buds { 6 } else { 2 };
        let tid: u16 = 2;
        drain_input(session, device, deadline, 8)?;
        let mut request = vec![0_u8; 64];
        request[..13].copy_from_slice(&[
            0x02,
            0x0C,
            0x01,
            0x00,
            0xFC,
            0x08,
            0x96,
            0xC3,
            0x41,
            0x04,
            0x01,
            (tid & 0xFF) as u8,
            (tid >> 8) as u8,
        ]);
        request[13] = checksum_sum(&request[6..=12]);
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(600),
            16,
            |b| {
                if b.len() < 15 || b[0] != 0x02 {
                    return Ok(false);
                }
                let len = usize::from(b[1]);
                if !(12..=62).contains(&len) || b.len() < len + 2 {
                    return Ok(false);
                }
                if b[2] != 0x04
                    || b[3] != 0xFF
                    || b[5] != 0x00
                    || b[6] != 0x96
                    || b[7] != 0xC3
                    || (b[8] >> 4) != 0x01
                    || b[9] != 0x04
                    || !matches!(b[10], 0x10 | 0xA0)
                    || u16::from_le_bytes([b[11], b[12]]) != tid
                {
                    return Ok(false);
                }
                if b[len + 1] != checksum_sum(&b[5..=len]) {
                    return Err(BatteryError::ChecksumFailed);
                }
                Ok(len + 1 >= 13 + min_payload)
            },
        )?;
        let len = usize::from(frame[1]);
        let payload = &frame[13..=len];
        if buds {
            let left = (payload[1] <= 100).then_some(payload[1]);
            let right = (payload[3] <= 100).then_some(payload[3]);
            let pct = match (left, right) {
                (Some(l), Some(r)) => Some(l.min(r)),
                (Some(l), None) => Some(l),
                (None, Some(r)) => Some(r),
                (None, None) => None,
            };
            if let Some(pct) = pct {
                Ok(reading(Some(pct), ChargeState::Unknown, None))
            } else {
                Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    ..reading(None, ChargeState::Unknown, None)
                })
            }
        } else if payload[1] == 0xFF {
            Ok(ProviderReading {
                reachability: Reachability::Offline,
                ..reading(None, ChargeState::Unknown, None)
            })
        } else {
            let pct = checked_percentage(payload[1])?;
            let charge = if payload[0] != 0 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            Ok(reading(Some(pct), charge, None))
        }
    }
}

struct SonyPulse3dProvider;

impl Provider for SonyPulse3dProvider {
    fn id(&self) -> &str {
        "sony.pulse_3d"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(1000),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Sony PULSE 3D".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = endpoint.vid == Some(0x054C)
            && endpoint.pid == Some(0x0D5E)
            && ((endpoint.usage_page == Some(0xFF01)
                && matches!(endpoint.usage, None | Some(0x0020)))
                || (endpoint.usage_page.is_none() && endpoint.interface_number == Some(3)));
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
        let mut newest: Option<Vec<u8>> = None;
        for _ in 0..64 {
            let timeout = deadline
                .remaining("Sony PULSE 3D")?
                .min(Duration::from_millis(50));
            match session.read_input(timeout) {
                Ok(report) => {
                    let f = frame(report);
                    if f.len() >= 8 && f[0] == 0xB0 {
                        newest = Some(f);
                    }
                }
                Err(TransportError::Timeout) => break,
                Err(error) => return Err(error.into_battery(&device.endpoint.id)),
            }
        }
        let r = match newest {
            Some(f) => f,
            None => wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(1000),
                16,
                |f| Ok(f.len() >= 8 && f[0] == 0xB0),
            )?,
        };
        if (r[4] & 0x08) == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                ..reading(None, ChargeState::Unknown, None)
            });
        }
        if r[3] == 0x80 {
            Ok(reading(None, ChargeState::Charging, None))
        } else if r[3] <= 100 {
            Ok(reading(Some(r[3]), ChargeState::Discharging, None))
        } else {
            Err(BatteryError::invalid_response(format!(
                "invalid Sony PULSE 3D battery byte 0x{:02X}",
                r[3]
            )))
        }
    }
}
