//! Minimal C-ABI bridge over `wireless-battery-tray-core`.
//!
//! Non-Rust native UI shells (such as macOS SwiftUI `MenuBarExtra`) link this
//! static library and exchange UTF-8 JSON snapshots without complex FFI structs.

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;
use std::sync::Arc;
use wireless_battery_tray_core::{PollerHandle, TrayConfig, TrayService, TrayUpdate};

/// Callback signature matching `WbTrayUpdateCallback` in `wireless_battery_tray.h`.
pub type WbTrayUpdateCallback = Option<unsafe extern "C" fn(*const c_char, *mut c_void)>;

/// Opaque handle exposed across the C ABI.
pub struct WbTrayHandle {
    poller: PollerHandle,
    callback: CallbackContext,
}

#[derive(serde::Serialize)]
struct UpdateEnvelope<'a> {
    snapshot: &'a wireless_battery_tray_core::TraySnapshot,
    alerts: &'a [wireless_battery_tray_core::LowBatteryAlert],
}

#[derive(Clone, Copy)]
struct CallbackContext {
    func: Option<unsafe extern "C" fn(*const c_char, *mut c_void)>,
    user_data: usize,
}

impl CallbackContext {
    fn invoke(self, update: &TrayUpdate) {
        let Some(func) = self.func else {
            return;
        };
        let envelope = UpdateEnvelope {
            snapshot: &update.snapshot,
            alerts: &update.alerts,
        };
        if let Ok(json) = serde_json::to_string(&envelope) {
            if let Ok(c_str) = CString::new(json) {
                unsafe {
                    func(c_str.as_ptr(), self.user_data as *mut c_void);
                }
            }
        }
    }
}

/// Create and start the tray service with an optional update callback.
#[no_mangle]
pub extern "C" fn wb_tray_create(
    callback: WbTrayUpdateCallback,
    user_data: *mut c_void,
) -> *mut WbTrayHandle {
    let Ok(service) = TrayService::new() else {
        return ptr::null_mut();
    };
    let ctx = CallbackContext {
        func: callback,
        user_data: user_data as usize,
    };
    let poller = PollerHandle::start(Arc::new(service), move |update| {
        ctx.invoke(&update);
    });
    Box::into_raw(Box::new(WbTrayHandle {
        poller,
        callback: ctx,
    }))
}

/// Stop polling and free the handle.
///
/// # Safety
/// `handle` must be NULL or a pointer previously returned by `wb_tray_create`
/// that has not already been destroyed.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_destroy(handle: *mut WbTrayHandle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// Return the current snapshot serialized as UTF-8 JSON.
///
/// # Safety
/// `handle` must be a valid pointer returned by `wb_tray_create`.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_snapshot_json(handle: *const WbTrayHandle) -> *mut c_char {
    let Some(handle) = (unsafe { handle.as_ref() }) else {
        return ptr::null_mut();
    };
    let snapshot = handle.poller.service().snapshot();
    to_c_string(serde_json::to_string(&snapshot).ok())
}

/// Request an asynchronous refresh of all eligible devices.
///
/// # Safety
/// `handle` must be a valid pointer returned by `wb_tray_create`.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_request_refresh(handle: *const WbTrayHandle) {
    if let Some(handle) = unsafe { handle.as_ref() } {
        handle.poller.request_refresh();
    }
}

/// Request an asynchronous refresh of a single device.
///
/// # Safety
/// `handle` must be valid and `device_id_utf8` must be a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_request_device_refresh(
    handle: *const WbTrayHandle,
    device_id_utf8: *const c_char,
) {
    let (Some(handle), Some(id)) = (unsafe { handle.as_ref() }, unsafe {
        read_c_str(device_id_utf8)
    }) else {
        return;
    };
    handle.poller.request_device_refresh(id);
}

/// Update and persist configuration from UTF-8 JSON.
///
/// # Safety
/// `handle` must be valid and `config_json_utf8` must be a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_update_config_json(
    handle: *const WbTrayHandle,
    config_json_utf8: *const c_char,
) -> bool {
    let (Some(handle), Some(raw)) = (unsafe { handle.as_ref() }, unsafe {
        read_c_str(config_json_utf8)
    }) else {
        return false;
    };
    let Ok(config) = serde_json::from_str::<TrayConfig>(&raw) else {
        return false;
    };
    let prev = handle.poller.service().config();
    let needs_repoll = prev.poll_interval_secs != config.poll_interval_secs
        || prev.allow_experimental != config.allow_experimental
        || prev.allow_stale != config.allow_stale
        || prev.read_timeout_ms != config.read_timeout_ms;
    let snapshot = if needs_repoll {
        handle.poller.update_config_async(config)
    } else {
        handle.poller.service().update_config_async(config)
    };
    handle.callback.invoke(&TrayUpdate {
        snapshot,
        alerts: Vec::new(),
    });
    true
}

/// Return sanitized passive diagnostic JSON for one device.
///
/// # Safety
/// `handle` must be valid and `device_id_utf8` must be a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_diagnose_json(
    handle: *const WbTrayHandle,
    device_id_utf8: *const c_char,
) -> *mut c_char {
    let (Some(handle), Some(id)) = (unsafe { handle.as_ref() }, unsafe {
        read_c_str(device_id_utf8)
    }) else {
        return ptr::null_mut();
    };
    to_c_string(handle.poller.service().diagnose_json(&id).ok())
}

/// Return sanitized error report JSON (including `last_error`) for one device.
///
/// # Safety
/// `handle` must be valid and `device_id_utf8` must be a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_error_report_json(
    handle: *const WbTrayHandle,
    device_id_utf8: *const c_char,
) -> *mut c_char {
    let (Some(handle), Some(id)) = (unsafe { handle.as_ref() }, unsafe {
        read_c_str(device_id_utf8)
    }) else {
        return ptr::null_mut();
    };
    to_c_string(handle.poller.service().error_report_json(&id).ok())
}

/// Free a UTF-8 string returned by this library.
///
/// # Safety
/// `ptr` must be NULL or a pointer previously allocated by this library.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}

unsafe fn read_c_str(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

fn to_c_string(value: Option<String>) -> *mut c_char {
    value
        .and_then(|s| CString::new(s).ok())
        .map_or(ptr::null_mut(), CString::into_raw)
}
