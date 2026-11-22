//! Development-only C-ABI hooks for macOS/native shells.
//!
//! Compiled strictly behind `#[cfg(feature = "dev-tools")]` so release builds
//! export zero dev symbols.

use crate::{ffi_catch, read_c_str, WbTrayHandle};
use std::ffi::c_char;
use wireless_battery::{ChargeState, ErrorKind, Freshness, Reachability};
use wireless_battery_tray_core::dev_tools::DevDeviceSpec;
use wireless_battery_tray_core::DeviceKind;

#[derive(serde::Deserialize)]
struct DevDevicePayload {
    name: String,
    kind: String,
    percentage: Option<u8>,
    state: String,
}

/// Toggle forced low-battery UI state and schedule an async callback on the poller thread.
///
/// # Safety
/// `handle` must be a valid pointer returned by `wb_tray_create`.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_dev_toggle_low_battery(handle: *const WbTrayHandle) -> bool {
    ffi_catch(false, || {
        let Some(handle) = (unsafe { handle.as_ref() }) else {
            return false;
        };
        let (forced, _) = handle.poller.service().dev_toggle_low_battery();
        handle.poller.request_snapshot_emit();
        forced
    })
}

/// Emit a synthetic low-battery alert from the background poller thread.
///
/// # Safety
/// `handle` must be a valid pointer returned by `wb_tray_create`.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_dev_send_low_battery_alert(
    handle: *const WbTrayHandle,
    percentage: u8,
) {
    ffi_catch((), || {
        let Some(handle) = (unsafe { handle.as_ref() }) else {
            return;
        };
        let alert = handle
            .poller
            .service()
            .dev_make_low_battery_alert(percentage);
        handle.poller.request_alert_emit(alert);
    });
}

/// Add a simulated peripheral from a UTF-8 JSON payload and push the updated snapshot.
///
/// # Safety
/// `handle` must be valid and `spec_json_utf8` must be a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_dev_add_device_json(
    handle: *const WbTrayHandle,
    spec_json_utf8: *const c_char,
) -> bool {
    ffi_catch(false, || {
        let (Some(handle), Some(raw)) = (unsafe { handle.as_ref() }, unsafe {
            read_c_str(spec_json_utf8)
        }) else {
            return false;
        };
        let Ok(payload) = serde_json::from_str::<DevDevicePayload>(&raw) else {
            return false;
        };
        let kind = match payload.kind.as_str() {
            "keyboard" => DeviceKind::Keyboard,
            "headset" => DeviceKind::Headset,
            "gamepad" => DeviceKind::Gamepad,
            _ => DeviceKind::Mouse,
        };
        let name = if payload.name.trim().is_empty() {
            "Dev Mock Device".to_owned()
        } else {
            payload.name.trim().to_owned()
        };
        let pct = payload.percentage.map(|p| p.min(100));
        let spec = match payload.state.as_str() {
            "charging" => DevDeviceSpec {
                name,
                kind,
                percentage: Some(pct.unwrap_or(50)),
                charge_state: ChargeState::Charging,
                reachability: Reachability::Online,
                freshness: Some(Freshness::Live),
                last_error: None,
            },
            "stale" => DevDeviceSpec {
                name,
                kind,
                percentage: Some(pct.unwrap_or(50)),
                charge_state: ChargeState::Discharging,
                reachability: Reachability::Sleeping,
                freshness: Some(Freshness::Stale),
                last_error: None,
            },
            "error" => DevDeviceSpec {
                name,
                kind,
                percentage: None,
                charge_state: ChargeState::Unknown,
                reachability: Reachability::Unknown,
                freshness: None,
                last_error: Some(ErrorKind::Timeout),
            },
            _ => DevDeviceSpec {
                name,
                kind,
                percentage: Some(pct.unwrap_or(50)),
                charge_state: ChargeState::Discharging,
                reachability: Reachability::Online,
                freshness: Some(Freshness::Live),
                last_error: None,
            },
        };
        let _ = handle.poller.service().dev_add_device(spec);
        handle.poller.request_snapshot_emit();
        true
    })
}

/// Clear all simulated devices and exit forced low-battery state.
///
/// # Safety
/// `handle` must be a valid pointer returned by `wb_tray_create`.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_dev_reset(handle: *const WbTrayHandle) {
    ffi_catch((), || {
        let Some(handle) = (unsafe { handle.as_ref() }) else {
            return;
        };
        let _ = handle.poller.service().dev_reset();
        handle.poller.request_snapshot_emit();
    });
}
