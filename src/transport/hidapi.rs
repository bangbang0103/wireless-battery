use super::{classify_hid_error, HidReport, HidSession, Transport, TransportError};
use crate::identity;
use crate::model::{Endpoint, EndpointId, HidBusType, TransportKind};
use hidapi::{BusType, HidApi, HidDevice};
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use std::time::Duration;

const MAX_REPORT_PAYLOAD: usize = 4_096;
const MAX_REPORT_DESCRIPTOR: usize = 4_096;

/// Real HIDAPI transport. Raw operating-system paths remain private.
pub struct HidApiTransport {
    api: Mutex<HidApi>,
    paths: RwLock<HashMap<EndpointId, String>>,
}

impl HidApiTransport {
    /// Initialize HIDAPI.
    pub fn new() -> Result<Self, TransportError> {
        let api = HidApi::new().map_err(|error| classify_hid_error(error.to_string()))?;
        Ok(Self {
            api: Mutex::new(api),
            paths: RwLock::new(HashMap::new()),
        })
    }
}

impl Transport for HidApiTransport {
    fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
        let mut api = self
            .api
            .lock()
            .map_err(|_| TransportError::Message("HIDAPI lock poisoned".to_owned()))?;
        api.refresh_devices()
            .map_err(|error| classify_hid_error(error.to_string()))?;
        let mut paths = self
            .paths
            .write()
            .map_err(|_| TransportError::Message("endpoint lock poisoned".to_owned()))?;
        let mut endpoints = Vec::new();
        let mut active = HashMap::new();
        for info in api.device_list() {
            let path = info.path().to_string_lossy().into_owned();
            let stable_key = if path.is_empty() {
                format!(
                    "hid:{:04x}:{:04x}:{:?}:{:?}",
                    info.vendor_id(),
                    info.product_id(),
                    info.usage_page(),
                    info.usage()
                )
            } else {
                format!("hid:{path}")
            };
            let id = identity::endpoint_id(&stable_key);
            active.insert(id.clone(), path);
            let interface_number = info.interface_number();
            let bus_type = match info.bus_type() {
                BusType::Usb => Some(HidBusType::Usb),
                BusType::Bluetooth => Some(HidBusType::Bluetooth),
                BusType::I2c => Some(HidBusType::I2c),
                BusType::Spi => Some(HidBusType::Spi),
                BusType::Unknown => None,
            };
            endpoints.push(Endpoint {
                id,
                transport: TransportKind::Hid,
                vid: Some(info.vendor_id()),
                pid: Some(info.product_id()),
                usage_page: Some(info.usage_page()),
                usage: Some(info.usage()),
                interface_number: (interface_number >= 0).then_some(interface_number),
                bus_type,
                input_report_length: None,
                output_report_length: None,
                feature_report_length: None,
                product_name: info.product_string().map(ToOwned::to_owned),
                manufacturer: info.manufacturer_string().map(ToOwned::to_owned),
                system_path: stable_key,
            });
        }
        *paths = active;
        Ok(endpoints)
    }

    fn open(&self, endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
        let path = self
            .paths
            .read()
            .map_err(|_| TransportError::Message("endpoint lock poisoned".to_owned()))?
            .get(endpoint)
            .cloned()
            .ok_or(TransportError::DeviceDisconnected)?;
        let c_path = std::ffi::CString::new(path.as_str())
            .map_err(|_| TransportError::Message("invalid operating-system HID path".to_owned()))?;
        let device = self
            .api
            .lock()
            .map_err(|_| TransportError::Message("HIDAPI lock poisoned".to_owned()))?
            .open_path(c_path.as_c_str())
            .map_err(|error| classify_hid_error(error.to_string()))?;
        let uses_numbered_reports = detect_numbered_reports(&device);
        Ok(Box::new(HidApiSession {
            device,
            uses_numbered_reports,
            #[cfg(windows)]
            path,
            #[cfg(windows)]
            windows: None,
        }))
    }
}

struct HidApiSession {
    device: HidDevice,
    uses_numbered_reports: bool,
    #[cfg(windows)]
    path: String,
    #[cfg(windows)]
    windows: Option<super::windows::WindowsHid>,
}

impl HidApiSession {
    fn with_report_id(report_id: u8, payload: &[u8]) -> Result<Vec<u8>, TransportError> {
        if payload.len() > MAX_REPORT_PAYLOAD {
            return Err(TransportError::InvalidResponse(
                "report exceeds payload limit".to_owned(),
            ));
        }
        let mut report = Vec::with_capacity(payload.len() + 1);
        report.push(report_id);
        report.extend_from_slice(payload);
        Ok(report)
    }
}

impl HidSession for HidApiSession {
    fn send_feature(&mut self, report_id: u8, payload: &[u8]) -> Result<(), TransportError> {
        let report = Self::with_report_id(report_id, payload)?;
        self.device
            .send_feature_report(&report)
            .map_err(|error| classify_hid_error(error.to_string()))?;
        Ok(())
    }

    fn get_feature(&mut self, report_id: u8, length: usize) -> Result<Vec<u8>, TransportError> {
        if length > MAX_REPORT_PAYLOAD {
            return Err(TransportError::InvalidResponse(
                "report exceeds payload limit".to_owned(),
            ));
        }
        let mut buffer = vec![0_u8; length + 1];
        buffer[0] = report_id;
        let count = self
            .device
            .get_feature_report(&mut buffer)
            .map_err(|error| classify_hid_error(error.to_string()))?;
        if count == 0 {
            return Err(TransportError::InvalidResponse(
                "empty feature response".to_owned(),
            ));
        }
        if buffer[0] != report_id {
            return Err(TransportError::InvalidResponse(
                "feature report ID mismatch".to_owned(),
            ));
        }
        Ok(buffer[1..count].to_vec())
    }

    fn write_output(&mut self, report_id: u8, payload: &[u8]) -> Result<(), TransportError> {
        let report = Self::with_report_id(report_id, payload)?;
        self.device
            .write(&report)
            .map_err(|error| classify_hid_error(error.to_string()))?;
        Ok(())
    }

    fn set_output(&mut self, report_id: u8, payload: &[u8]) -> Result<(), TransportError> {
        let report = Self::with_report_id(report_id, payload)?;
        #[cfg(windows)]
        {
            let mut report = report;
            if self.windows.is_none() {
                self.windows = Some(super::windows::WindowsHid::open(&self.path)?);
            }
            self.windows
                .as_ref()
                .ok_or_else(|| {
                    TransportError::Message("Windows HID handle unavailable".to_owned())
                })?
                .set_output(&mut report)
        }
        #[cfg(not(windows))]
        {
            let _ = report;
            Err(TransportError::Unsupported(
                "SET_OUTPUT_REPORT is currently implemented only on Windows".to_owned(),
            ))
        }
    }

    fn read_input(&mut self, timeout: Duration) -> Result<HidReport, TransportError> {
        let milliseconds = if timeout.is_zero() {
            0
        } else {
            timeout.as_millis().clamp(1, i32::MAX as u128) as i32
        };
        let mut buffer = vec![0_u8; MAX_REPORT_PAYLOAD + 1];
        let count = self
            .device
            .read_timeout(&mut buffer, milliseconds)
            .map_err(|error| classify_hid_error(error.to_string()))?;
        if count == 0 {
            return Err(TransportError::Timeout);
        }
        if self.uses_numbered_reports {
            Ok(HidReport {
                report_id: buffer[0],
                data: buffer[1..count].to_vec(),
            })
        } else {
            Ok(HidReport {
                report_id: 0,
                data: buffer[..count].to_vec(),
            })
        }
    }

    fn get_input_report(
        &mut self,
        report_id: u8,
        length: usize,
    ) -> Result<Vec<u8>, TransportError> {
        if length > MAX_REPORT_PAYLOAD {
            return Err(TransportError::InvalidResponse(
                "report exceeds payload limit".to_owned(),
            ));
        }
        let mut buffer = vec![0_u8; length + 1];
        buffer[0] = report_id;
        #[cfg(windows)]
        {
            if self.windows.is_none() {
                self.windows = Some(super::windows::WindowsHid::open(&self.path)?);
            }
            self.windows
                .as_ref()
                .ok_or_else(|| {
                    TransportError::Message("Windows HID handle unavailable".to_owned())
                })?
                .get_input(&mut buffer)?;
            if buffer[0] != report_id {
                return Err(TransportError::InvalidResponse(
                    "input report ID mismatch".to_owned(),
                ));
            }
            Ok(buffer[1..].to_vec())
        }
        #[cfg(not(windows))]
        {
            let _ = buffer;
            Err(TransportError::Unsupported(
                "GET_INPUT_REPORT is currently implemented only on Windows".to_owned(),
            ))
        }
    }
}

fn detect_numbered_reports(device: &HidDevice) -> bool {
    let mut descriptor = vec![0_u8; MAX_REPORT_DESCRIPTOR];
    match device.get_report_descriptor(&mut descriptor) {
        Ok(length) if length > 0 => descriptor_uses_numbered_reports(&descriptor[..length]),
        // Fall back to assuming numbered reports when descriptor query is unsupported.
        _ => true,
    }
}

fn descriptor_uses_numbered_reports(descriptor: &[u8]) -> bool {
    let mut index = 0;
    while index < descriptor.len() {
        let header = descriptor[index];
        if header == 0xFE {
            let Some(&size) = descriptor.get(index + 1) else {
                break;
            };
            index = index.saturating_add(3).saturating_add(usize::from(size));
            continue;
        }
        let data_len = match header & 0x03 {
            3 => 4,
            other => usize::from(other),
        };
        // Global item tag 8 (0x84) is Report ID.
        if (header & 0xFC) == 0x84 && data_len >= 1 {
            if let Some(&report_id) = descriptor.get(index + 1) {
                if report_id != 0 {
                    return true;
                }
            }
        }
        index = index.saturating_add(1).saturating_add(data_len);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::descriptor_uses_numbered_reports;

    #[test]
    fn detects_numbered_and_unnumbered_hid_descriptors() {
        // Usage Page (Vendor), Usage (1), Collection (App), Report ID (4), End Collection
        let numbered = [0x06, 0x00, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x04, 0xC0];
        assert!(descriptor_uses_numbered_reports(&numbered));

        // Same descriptor without Report ID item (0x85)
        let unnumbered = [0x06, 0x00, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0xC0];
        assert!(!descriptor_uses_numbered_reports(&unnumbered));
    }
}
