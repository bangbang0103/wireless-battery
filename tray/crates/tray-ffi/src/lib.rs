//! Minimal C-ABI bridge over `wireless-battery-tray-core`.
//!
//! Non-Rust native UI shells (such as the macOS AppKit/SwiftUI shell) link this
//! static library and exchange UTF-8 JSON snapshots without complex FFI structs.

use std::ffi::{c_char, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::Arc;
use wireless_battery_tray_core::{
    evaluate_release_json, format_footer_summary_rfc3339, format_poll_label,
    format_relative_time_rfc3339, is_valid_update_url, PollerHandle, TrayConfig, TrayService,
    TrayUpdate, UpdateCheckResult, LATEST_RELEASE_API_URL, RELEASES_URL,
};

#[cfg(feature = "dev-tools")]
mod dev_tools;

/// Callback signature matching `WbTrayUpdateCallback` in `wireless_battery_tray.h`.
pub type WbTrayUpdateCallback = Option<unsafe extern "C" fn(*const c_char, *mut c_void)>;

/// Opaque handle exposed across the C ABI.
pub struct WbTrayHandle {
    pub(crate) poller: PollerHandle,
}

#[derive(serde::Serialize)]
struct UpdateEnvelope<'a> {
    snapshot: &'a wireless_battery_tray_core::TraySnapshot,
    alerts: &'a [wireless_battery_tray_core::LowBatteryAlert],
}

#[derive(serde::Serialize)]
struct ReleaseEvaluationPayload {
    status: &'static str,
    latest_version: Option<String>,
    url: Option<String>,
    message: Option<String>,
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

pub(crate) fn ffi_catch<T, F>(default: T, f: F) -> T
where
    F: FnOnce() -> T,
{
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(default)
}

/// Create and start the tray service with an optional update callback.
#[no_mangle]
pub extern "C" fn wb_tray_create(
    callback: WbTrayUpdateCallback,
    user_data: *mut c_void,
) -> *mut WbTrayHandle {
    ffi_catch(ptr::null_mut(), || {
        let Ok(service) = TrayService::new() else {
            return ptr::null_mut();
        };
        let ctx = CallbackContext {
            func: callback,
            user_data: user_data as usize,
        };
        let Ok(poller) = PollerHandle::start(Arc::new(service), move |update| {
            ctx.invoke(&update);
        }) else {
            return ptr::null_mut();
        };
        Box::into_raw(Box::new(WbTrayHandle { poller }))
    })
}

/// Stop polling and free the handle.
///
/// # Safety
/// `handle` must be NULL or a pointer previously returned by `wb_tray_create`
/// that has not already been destroyed.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_destroy(handle: *mut WbTrayHandle) {
    ffi_catch((), || {
        if !handle.is_null() {
            drop(unsafe { Box::from_raw(handle) });
        }
    });
}

/// Inform the service whether the host OS UI language resolves to Simplified Chinese.
///
/// # Safety
/// `handle` must be NULL or a valid pointer returned by `wb_tray_create`.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_set_system_language_zh(handle: *const WbTrayHandle, is_zh: bool) {
    ffi_catch((), || {
        if let Some(handle) = unsafe { handle.as_ref() } {
            handle.poller.service().set_system_is_zh(is_zh);
            handle.poller.request_snapshot_emit();
        }
    });
}

/// Return the current snapshot serialized as UTF-8 JSON.
///
/// # Safety
/// `handle` must be a valid pointer returned by `wb_tray_create`.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_snapshot_json(handle: *const WbTrayHandle) -> *mut c_char {
    ffi_catch(ptr::null_mut(), || {
        let Some(handle) = (unsafe { handle.as_ref() }) else {
            return ptr::null_mut();
        };
        let snapshot = handle.poller.service().snapshot();
        to_c_string(serde_json::to_string(&snapshot).ok())
    })
}

/// Request an asynchronous refresh of all eligible devices.
///
/// # Safety
/// `handle` must be a valid pointer returned by `wb_tray_create`.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_request_refresh(handle: *const WbTrayHandle) {
    ffi_catch((), || {
        if let Some(handle) = unsafe { handle.as_ref() } {
            handle.poller.request_refresh();
        }
    });
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
    ffi_catch((), || {
        let (Some(handle), Some(id)) = (unsafe { handle.as_ref() }, unsafe {
            read_c_str(device_id_utf8)
        }) else {
            return;
        };
        handle.poller.request_device_refresh(id);
    });
}

/// Update and persist configuration from UTF-8 JSON.
///
/// Updated snapshots are emitted strictly from the background poller thread.
///
/// # Safety
/// `handle` must be valid and `config_json_utf8` must be a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_update_config_json(
    handle: *const WbTrayHandle,
    config_json_utf8: *const c_char,
) -> bool {
    ffi_catch(false, || {
        let (Some(handle), Some(raw)) = (unsafe { handle.as_ref() }, unsafe {
            read_c_str(config_json_utf8)
        }) else {
            return false;
        };
        let Ok(config) = serde_json::from_str::<TrayConfig>(&raw) else {
            return false;
        };
        let next = config.normalized();
        let _ = handle.poller.update_config_async(next);
        handle.poller.request_snapshot_emit();
        true
    })
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
    ffi_catch(ptr::null_mut(), || {
        let (Some(handle), Some(id)) = (unsafe { handle.as_ref() }, unsafe {
            read_c_str(device_id_utf8)
        }) else {
            return ptr::null_mut();
        };
        to_c_string(handle.poller.service().diagnose_json(&id).ok())
    })
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
    ffi_catch(ptr::null_mut(), || {
        let (Some(handle), Some(id)) = (unsafe { handle.as_ref() }, unsafe {
            read_c_str(device_id_utf8)
        }) else {
            return ptr::null_mut();
        };
        to_c_string(handle.poller.service().error_report_json(&id).ok())
    })
}

/// Check whether an external update URL uses the permitted GitHub HTTPS origin.
///
/// # Safety
/// `url_utf8` must be NULL or a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_is_valid_update_url(url_utf8: *const c_char) -> bool {
    ffi_catch(false, || {
        let Some(url) = (unsafe { read_c_str(url_utf8) }) else {
            return false;
        };
        is_valid_update_url(&url)
    })
}

/// Parse a GitHub release JSON body against `current_version_utf8` and serialize result JSON.
///
/// # Safety
/// Both arguments must be valid NUL-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_evaluate_release_json(
    body_json_utf8: *const c_char,
    current_version_utf8: *const c_char,
) -> *mut c_char {
    ffi_catch(ptr::null_mut(), || {
        let (Some(body), Some(current)) = (unsafe { read_c_str(body_json_utf8) }, unsafe {
            read_c_str(current_version_utf8)
        }) else {
            return ptr::null_mut();
        };
        let payload = match evaluate_release_json(&body, &current) {
            UpdateCheckResult::Available {
                latest_version,
                url,
            } => ReleaseEvaluationPayload {
                status: "available",
                latest_version: Some(latest_version),
                url: Some(url),
                message: None,
            },
            UpdateCheckResult::UpToDate { current_version } => ReleaseEvaluationPayload {
                status: "up_to_date",
                latest_version: Some(current_version),
                url: None,
                message: None,
            },
            UpdateCheckResult::Failed(msg) => ReleaseEvaluationPayload {
                status: "failed",
                latest_version: None,
                url: None,
                message: Some(msg),
            },
        };
        to_c_string(serde_json::to_string(&payload).ok())
    })
}

/// Return the GitHub Releases latest API URL as a newly allocated UTF-8 string.
#[no_mangle]
pub extern "C" fn wb_tray_latest_release_api_url() -> *mut c_char {
    ffi_catch(ptr::null_mut(), || {
        to_c_string(Some(LATEST_RELEASE_API_URL.to_owned()))
    })
}

/// Return the GitHub Releases download page URL as a newly allocated UTF-8 string.
#[no_mangle]
pub extern "C" fn wb_tray_releases_url() -> *mut c_char {
    ffi_catch(ptr::null_mut(), || {
        to_c_string(Some(RELEASES_URL.to_owned()))
    })
}

/// Format an optional RFC3339 timestamp relative to now.
///
/// # Safety
/// `iso8601_utf8` may be NULL or a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_format_relative_time(
    iso8601_utf8: *const c_char,
    is_zh: bool,
) -> *mut c_char {
    ffi_catch(ptr::null_mut(), || {
        let raw = unsafe { read_c_str(iso8601_utf8) };
        let opt = raw.as_deref().filter(|s| !s.is_empty());
        to_c_string(Some(format_relative_time_rfc3339(opt, is_zh)))
    })
}

/// Format a polling interval in seconds for Settings UI dropdown labels.
#[no_mangle]
pub extern "C" fn wb_tray_format_poll_label(secs: u64, is_zh: bool) -> *mut c_char {
    ffi_catch(ptr::null_mut(), || {
        to_c_string(Some(format_poll_label(secs, is_zh)))
    })
}

/// Format the footer summary (`version · sync · cadence`).
///
/// # Safety
/// `app_version_utf8` must be valid UTF-8; `last_sync_iso8601_utf8` may be NULL or valid UTF-8.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_format_footer_summary(
    app_version_utf8: *const c_char,
    last_sync_iso8601_utf8: *const c_char,
    poll_secs: u64,
    is_zh: bool,
) -> *mut c_char {
    ffi_catch(ptr::null_mut(), || {
        let version = unsafe { read_c_str(app_version_utf8) }.unwrap_or_default();
        let raw = unsafe { read_c_str(last_sync_iso8601_utf8) };
        let opt = raw.as_deref().filter(|s| !s.is_empty());
        to_c_string(Some(format_footer_summary_rfc3339(
            &version, opt, poll_secs, is_zh,
        )))
    })
}

/// Free a UTF-8 string returned by this library.
///
/// # Safety
/// `ptr` must be NULL or a pointer previously allocated by this library.
#[no_mangle]
pub unsafe extern "C" fn wb_tray_string_free(ptr: *mut c_char) {
    ffi_catch((), || {
        if !ptr.is_null() {
            drop(unsafe { CString::from_raw(ptr) });
        }
    });
}

pub(crate) unsafe fn read_c_str(ptr: *const c_char) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use wireless_battery_tray_core::{TrayConfig, TrayService};

    #[test]
    fn null_safety_and_invalid_utf8_do_not_crash() {
        unsafe {
            assert!(wb_tray_snapshot_json(ptr::null()).is_null());
            wb_tray_request_refresh(ptr::null());
            wb_tray_request_device_refresh(ptr::null(), ptr::null());
            assert!(!wb_tray_update_config_json(ptr::null(), ptr::null()));
            assert!(wb_tray_diagnose_json(ptr::null(), ptr::null()).is_null());
            assert!(wb_tray_error_report_json(ptr::null(), ptr::null()).is_null());
            wb_tray_string_free(ptr::null_mut());
            wb_tray_destroy(ptr::null_mut());

            let bad_utf8 = [0xffu8 as c_char, 0];
            assert!(read_c_str(bad_utf8.as_ptr()).is_none());
            assert!(!wb_tray_is_valid_update_url(bad_utf8.as_ptr()));
            assert!(wb_tray_evaluate_release_json(ptr::null(), bad_utf8.as_ptr()).is_null());
        }
    }

    #[test]
    fn shared_formatters_and_release_eval_match_expectations() {
        unsafe {
            let ok_url =
                CString::new("https://github.com/bangbang0103/wireless-battery/releases").unwrap();
            let bad_url = CString::new("https://evil.example.com/phish").unwrap();
            assert!(wb_tray_is_valid_update_url(ok_url.as_ptr()));
            assert!(!wb_tray_is_valid_update_url(bad_url.as_ptr()));

            let body =
                CString::new(r#"{"tag_name":"v9.9.9","html_url":"https://evil.example.com/bad"}"#)
                    .unwrap();
            let ver = CString::new("0.1.0").unwrap();
            let eval_ptr = wb_tray_evaluate_release_json(body.as_ptr(), ver.as_ptr());
            assert!(!eval_ptr.is_null());
            let eval_str = CStr::from_ptr(eval_ptr).to_str().unwrap().to_owned();
            wb_tray_string_free(eval_ptr);
            let parsed: serde_json::Value = serde_json::from_str(&eval_str).unwrap();
            assert_eq!(parsed["status"], "available");
            assert_eq!(parsed["latest_version"], "v9.9.9");
            assert_eq!(
                parsed["url"],
                "https://github.com/bangbang0103/wireless-battery/releases"
            );

            let api_ptr = wb_tray_latest_release_api_url();
            assert_eq!(
                CStr::from_ptr(api_ptr).to_str().unwrap(),
                LATEST_RELEASE_API_URL
            );
            wb_tray_string_free(api_ptr);

            let rel_url_ptr = wb_tray_releases_url();
            assert_eq!(CStr::from_ptr(rel_url_ptr).to_str().unwrap(), RELEASES_URL);
            wb_tray_string_free(rel_url_ptr);

            let poll_ptr = wb_tray_format_poll_label(120, true);
            assert_eq!(CStr::from_ptr(poll_ptr).to_str().unwrap(), "2 分钟");
            wb_tray_string_free(poll_ptr);

            let rel_ptr = wb_tray_format_relative_time(ptr::null(), false);
            assert_eq!(CStr::from_ptr(rel_ptr).to_str().unwrap(), "Not synced");
            wb_tray_string_free(rel_ptr);
        }
    }

    #[test]
    fn update_config_emits_callback_from_background_poller_thread() {
        use wireless_battery::registry::Registry;
        use wireless_battery::{
            BatteryClient, ClientConfig, Endpoint, EndpointId, HidSession, Transport,
            TransportError,
        };

        struct EmptyTransport;
        impl Transport for EmptyTransport {
            fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
                Ok(Vec::new())
            }
            fn open(&self, _endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
                Err(TransportError::Timeout)
            }
        }

        let caller_tid = thread::current().id();
        let (tx, rx) = mpsc::channel::<(thread::ThreadId, String)>();
        unsafe extern "C" fn capture_cb(json: *const c_char, user_data: *mut c_void) {
            let sender = &*(user_data as *const mpsc::Sender<(thread::ThreadId, String)>);
            let s = CStr::from_ptr(json).to_string_lossy().into_owned();
            let _ = sender.send((thread::current().id(), s));
        }

        let client = BatteryClient::with_transport(
            ClientConfig {
                default_timeout: Duration::from_secs(1),
                enable_cache: true,
            },
            Arc::new(EmptyTransport),
            Registry::new(Vec::new()).unwrap(),
        )
        .unwrap();
        let service = Arc::new(TrayService::with_client(
            Arc::new(client),
            TrayConfig::default(),
            None,
        ));
        let ctx = CallbackContext {
            func: Some(capture_cb),
            user_data: (&tx as *const mpsc::Sender<(thread::ThreadId, String)>) as usize,
        };
        let poller = PollerHandle::start(service, move |upd| ctx.invoke(&upd)).unwrap();
        let handle = Box::into_raw(Box::new(WbTrayHandle { poller }));

        // Wait for the initial refresh cycle to complete (`refreshing == false`) so we don't
        // race with its final callback before testing config update delivery.
        loop {
            let (tid1, raw1) = rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_ne!(tid1, caller_tid);
            let parsed1: serde_json::Value = serde_json::from_str(&raw1).unwrap();
            if parsed1["snapshot"]["refreshing"] == false {
                break;
            }
        }

        let cfg = TrayConfig {
            poll_interval_secs: 900,
            ..Default::default()
        };
        let cfg_cstr = CString::new(serde_json::to_string(&cfg).unwrap()).unwrap();
        let bad_json = CString::new("{invalid").unwrap();
        unsafe {
            assert!(!wb_tray_update_config_json(handle, bad_json.as_ptr()));
            assert!(wb_tray_update_config_json(handle, cfg_cstr.as_ptr()));
        }

        let (tid2, body) = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_ne!(tid2, caller_tid);
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["snapshot"]["config"]["poll_interval_secs"], 900);

        unsafe {
            wb_tray_destroy(handle);
        }
    }

    #[test]
    fn c_header_matches_all_exported_signatures() {
        use std::collections::BTreeMap;

        fn normalize_ws(input: &str) -> String {
            input.split_whitespace().collect::<Vec<_>>().join(" ")
        }

        fn rust_ty_to_c(ty: &str) -> String {
            match normalize_ws(ty).as_str() {
                "" | "()" => "void".to_owned(),
                "bool" => "bool".to_owned(),
                "u8" => "uint8_t".to_owned(),
                "u64" => "uint64_t".to_owned(),
                "*mut c_char" => "char *".to_owned(),
                "*const c_char" => "const char *".to_owned(),
                "*mut c_void" => "void *".to_owned(),
                "*mut WbTrayHandle" => "WbTrayHandle *".to_owned(),
                "*const WbTrayHandle" => "const WbTrayHandle *".to_owned(),
                "WbTrayUpdateCallback" => "WbTrayUpdateCallback".to_owned(),
                other => panic!("unmapped Rust FFI type `{other}`; update header signature check"),
            }
        }

        fn normalize_c_type(raw: &str) -> String {
            let spaced = raw.replace('*', " * ");
            let tokens: Vec<&str> = spaced.split_whitespace().collect();
            let mut out = String::new();
            for (idx, tok) in tokens.iter().enumerate() {
                if idx > 0 {
                    out.push(' ');
                }
                out.push_str(tok);
            }
            out
        }

        fn strip_c_param_name(param: &str) -> String {
            let norm = normalize_c_type(param);
            if norm == "void" || norm.is_empty() {
                return String::new();
            }
            let mut tokens: Vec<&str> = norm.split_whitespace().collect();
            if tokens.len() >= 2 && tokens.last() != Some(&"*") {
                tokens.pop();
            }
            tokens.join(" ")
        }

        fn strip_c_comments(src: &str) -> String {
            let bytes = src.as_bytes();
            let mut out = String::with_capacity(src.len());
            let mut i = 0;
            while i < bytes.len() {
                if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                    i += 2;
                    while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                        i += 1;
                    }
                    i = (i + 2).min(bytes.len());
                } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                } else {
                    out.push(bytes[i] as char);
                    i += 1;
                }
            }
            out
        }

        fn parse_rust_exports(src: &str) -> BTreeMap<String, (String, Vec<String>)> {
            let mut map = BTreeMap::new();
            let mut cursor = 0;
            while cursor < src.len() {
                let tail = &src[cursor..];
                let next_unsafe = tail.find("pub unsafe extern \"C\" fn ");
                let next_safe = tail.find("pub extern \"C\" fn ");
                let (offset, prefix_len) = match (next_unsafe, next_safe) {
                    (Some(a), Some(b)) if a < b => (a, "pub unsafe extern \"C\" fn ".len()),
                    (Some(a), None) => (a, "pub unsafe extern \"C\" fn ".len()),
                    (_, Some(b)) => (b, "pub extern \"C\" fn ".len()),
                    (None, None) => break,
                };
                let start = cursor + offset + prefix_len;
                let rest = &src[start..];
                let brace = rest.find('{').expect("extern C fn missing body brace");
                let sig = &rest[..brace];
                let lparen = sig.find('(').expect("extern C fn missing '('");
                let rparen = sig.rfind(')').expect("extern C fn missing ')'");
                let name = sig[..lparen].trim().to_owned();
                let params_raw = sig[lparen + 1..rparen].trim();
                let params: Vec<String> = if params_raw.is_empty() {
                    Vec::new()
                } else {
                    params_raw
                        .split(',')
                        .map(str::trim)
                        .filter(|p| !p.is_empty())
                        .map(|p| {
                            let ty = p.split_once(':').expect("param missing ':'").1;
                            rust_ty_to_c(ty)
                        })
                        .collect()
                };
                let ret_raw = sig[rparen + 1..].trim();
                let ret = if let Some(after_arrow) = ret_raw.strip_prefix("->") {
                    rust_ty_to_c(after_arrow)
                } else {
                    "void".to_owned()
                };
                assert!(
                    map.insert(name.clone(), (ret, params)).is_none(),
                    "duplicate Rust extern C symbol `{name}`"
                );
                cursor = start + brace + 1;
            }
            map
        }

        fn parse_c_header(header: &str) -> BTreeMap<String, (String, Vec<String>)> {
            let uncommented = strip_c_comments(header);
            let cleaned: String = uncommented
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");
            let mut map = BTreeMap::new();
            for raw_stmt in cleaned.split(';') {
                let stmt = normalize_ws(raw_stmt);
                if stmt.is_empty()
                    || stmt.starts_with("typedef ")
                    || stmt.starts_with("extern \"C\"")
                    || !stmt.contains("wb_tray_")
                {
                    continue;
                }
                let stmt = stmt
                    .trim_start_matches("extern \"C\" {")
                    .trim_start_matches('{')
                    .trim();
                let lparen = stmt.find('(').expect("C decl missing '('");
                let rparen = stmt.rfind(')').expect("C decl missing ')'");
                let before = stmt[..lparen].trim();
                let name_pos = before
                    .rfind("wb_tray_")
                    .expect("C decl missing wb_tray_ name");
                let name = before[name_pos..].trim().to_owned();
                let ret = normalize_c_type(&before[..name_pos]);
                let params_raw = stmt[lparen + 1..rparen].trim();
                let params: Vec<String> = if params_raw.is_empty() || params_raw == "void" {
                    Vec::new()
                } else {
                    params_raw
                        .split(',')
                        .map(strip_c_param_name)
                        .filter(|p| !p.is_empty())
                        .collect()
                };
                assert!(
                    map.insert(name.clone(), (ret, params)).is_none(),
                    "duplicate C header declaration `{name}`"
                );
            }
            map
        }

        let mut rust_exports = parse_rust_exports(include_str!("lib.rs"));
        rust_exports.extend(parse_rust_exports(include_str!("dev_tools.rs")));
        let c_decls = parse_c_header(include_str!("../include/wireless_battery_tray.h"));

        assert_eq!(
            rust_exports, c_decls,
            "wireless_battery_tray.h signatures drifted from Rust extern \"C\" exports"
        );
    }
}
