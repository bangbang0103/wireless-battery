//! Windows XInput slot transport. XInput exposes four discrete levels, not percentages.

use super::{TransportError, XInputReport};
#[cfg(windows)]
use crate::identity;
#[cfg(windows)]
use crate::model::TransportKind;
use crate::model::{Endpoint, EndpointId};
use std::collections::HashMap;
use std::sync::RwLock;

/// XInput transport with four OS-managed slots.
pub struct XInputTransport {
    slots: RwLock<HashMap<EndpointId, u32>>,
}

impl XInputTransport {
    /// Construct without loading any device driver explicitly.
    pub fn new() -> Self {
        Self {
            slots: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) fn enumerate_slots(&self) -> Result<Vec<Endpoint>, TransportError> {
        #[cfg(windows)]
        let (active, endpoints) = {
            let mut active = HashMap::new();
            let mut endpoints = Vec::new();
            for slot in 0..4_u32 {
                if native::connected(slot) {
                    let stable = format!("xinput:{slot}");
                    let id = identity::endpoint_id(&stable);
                    active.insert(id.clone(), slot);
                    endpoints.push(Endpoint {
                        id,
                        transport: TransportKind::XInput,
                        vid: None,
                        pid: None,
                        usage_page: None,
                        usage: None,
                        interface_number: None,
                        bus_type: None,
                        input_report_length: None,
                        output_report_length: None,
                        feature_report_length: None,
                        product_name: Some(format!("Xbox controller {}", slot + 1)),
                        manufacturer: Some("XInput".to_owned()),
                        system_path: stable,
                    });
                }
            }
            (active, endpoints)
        };
        #[cfg(not(windows))]
        let (active, endpoints) = {
            // XInput is a Windows-only operating-system API.
            (HashMap::new(), Vec::new())
        };
        *self
            .slots
            .write()
            .map_err(|_| TransportError::Message("XInput slot lock poisoned".to_owned()))? = active;
        Ok(endpoints)
    }

    pub(crate) fn read_slot(
        &self,
        endpoint: &EndpointId,
    ) -> Result<Option<XInputReport>, TransportError> {
        let slot = self
            .slots
            .read()
            .map_err(|_| TransportError::Message("XInput slot lock poisoned".to_owned()))?
            .get(endpoint)
            .copied();
        #[cfg(windows)]
        if let Some(slot) = slot {
            return native::battery(slot).map(Some);
        }
        #[cfg(not(windows))]
        let _ = slot;
        Ok(None)
    }
}

impl Default for XInputTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod native {
    use super::TransportError;
    use crate::transport::XInputReport;
    use std::ffi::c_void;
    use std::sync::OnceLock;

    const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;

    #[repr(C)]
    struct BatteryInfo {
        battery_type: u8,
        battery_level: u8,
    }

    #[repr(C)]
    struct XInputState {
        packet_number: u32,
        gamepad: [u8; 12],
    }

    #[derive(Clone, Copy)]
    struct Api {
        state: usize,
        battery: usize,
    }

    static API: OnceLock<Option<Api>> = OnceLock::new();

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryExW(name: *const u16, file: isize, flags: u32) -> isize;
        fn FreeLibrary(module: isize) -> i32;
        fn GetProcAddress(module: isize, name: *const u8) -> usize;
    }

    fn api() -> Option<Api> {
        *API.get_or_init(|| {
            for library in ["XInput1_4.dll", "XInput1_3.dll"] {
                let wide = library
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect::<Vec<_>>();
                // SAFETY: the library name is NUL-terminated and restricted to System32;
                // a successfully resolved system DLL remains loaded for process lifetime.
                let module =
                    unsafe { LoadLibraryExW(wide.as_ptr(), 0, LOAD_LIBRARY_SEARCH_SYSTEM32) };
                if module == 0 {
                    continue;
                }
                // SAFETY: names are static NUL-terminated ASCII strings and module is valid.
                let state = unsafe { GetProcAddress(module, b"XInputGetState\0".as_ptr()) };
                let battery =
                    unsafe { GetProcAddress(module, b"XInputGetBatteryInformation\0".as_ptr()) };
                if state != 0 && battery != 0 {
                    return Some(Api { state, battery });
                }
                // SAFETY: module was just loaded above and will not be used further.
                unsafe {
                    FreeLibrary(module);
                }
            }
            None
        })
    }

    pub(super) fn connected(slot: u32) -> bool {
        let Some(api) = api() else {
            return false;
        };
        let mut state = XInputState {
            packet_number: 0,
            gamepad: [0_u8; 12],
        };
        // SAFETY: the symbol was resolved from XInput with the documented ABI and
        // `XInputState` matches the 16-byte, 4-byte-aligned Win32 `XINPUT_STATE` layout.
        let function: extern "system" fn(u32, *mut c_void) -> u32 =
            unsafe { std::mem::transmute(api.state) };
        function(slot, (&mut state as *mut XInputState).cast()) == 0
    }

    pub(super) fn battery(slot: u32) -> Result<XInputReport, TransportError> {
        let Some(api) = api() else {
            return Err(TransportError::Unsupported(
                "XInput1_4.dll and XInput1_3.dll are unavailable".to_owned(),
            ));
        };
        if !connected(slot) {
            return Err(TransportError::DeviceDisconnected);
        }
        let mut info = BatteryInfo {
            battery_type: 0,
            battery_level: 0,
        };
        // SAFETY: the resolved function has the documented XInput ABI and info has
        // the exact two-byte C layout; device type zero selects the gamepad.
        let function: extern "system" fn(u32, u8, *mut BatteryInfo) -> u32 =
            unsafe { std::mem::transmute(api.battery) };
        let result = function(slot, 0, &mut info);
        if result != 0 {
            return Err(TransportError::Message(format!(
                "XInput battery query failed with code {result}"
            )));
        }
        Ok(XInputReport {
            battery_type: info.battery_type,
            battery_level: info.battery_level,
        })
    }
}
