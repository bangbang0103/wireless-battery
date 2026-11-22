//! Lightweight native WinHTTP GitHub Releases update checker and browser opener.

use windows::core::{w, HSTRING};
use windows::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use wireless_battery_tray_core::{
    evaluate_release_json, is_valid_update_url, UpdateCheckResult, APP_VERSION, GITHUB_API_HOST,
    LATEST_RELEASE_API_PATH,
};

const MAX_RESPONSE_BYTES: usize = 512 * 1024;

/// Query GitHub Releases latest endpoint via WinHTTP (blocking; run on a background thread).
pub fn check_latest_release() -> UpdateCheckResult {
    match fetch_latest_release_body() {
        Ok(body) => evaluate_release_json(&body, APP_VERSION),
        Err(msg) => UpdateCheckResult::Failed(msg),
    }
}

/// Open an HTTPS GitHub URL in the user's default web browser.
pub fn open_url(url: &str) {
    if !is_valid_update_url(url) {
        return;
    }
    let wide_url = HSTRING::from(url);
    unsafe {
        let _ = ShellExecuteW(None, w!("open"), &wide_url, None, None, SW_SHOWNORMAL);
    }
}

struct WinHttpHandle(*mut std::ffi::c_void);

impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
}

fn fetch_latest_release_body() -> Result<String, String> {
    unsafe {
        let user_agent = HSTRING::from(format!("wireless-battery-tray/{APP_VERSION}"));
        let session = WinHttpOpen(
            &user_agent,
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            None,
            None,
            0,
        );
        if session.is_null() {
            return Err("WinHttpOpen failed".to_owned());
        }
        let _session = WinHttpHandle(session);
        let _ = WinHttpSetTimeouts(session, 4_000, 4_000, 5_000, 5_000);

        let host = HSTRING::from(GITHUB_API_HOST);
        let connect = WinHttpConnect(session, &host, 443, 0);
        if connect.is_null() {
            return Err("WinHttpConnect failed".to_owned());
        }
        let _connect = WinHttpHandle(connect);

        let path = HSTRING::from(LATEST_RELEASE_API_PATH);
        let request = WinHttpOpenRequest(
            connect,
            w!("GET"),
            &path,
            None,
            None,
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        );
        if request.is_null() {
            return Err("WinHttpOpenRequest failed".to_owned());
        }
        let _request = WinHttpHandle(request);

        let headers = w!("Accept: application/vnd.github+json\r\n");
        if WinHttpSendRequest(request, Some(headers.as_wide()), None, 0, 0, 0).is_err() {
            return Err("network request failed".to_owned());
        }
        if WinHttpReceiveResponse(request, std::ptr::null_mut()).is_err() {
            return Err("no response from server".to_owned());
        }

        let mut status_code: u32 = 0;
        let mut status_size = std::mem::size_of::<u32>() as u32;
        if WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            None,
            Some((&mut status_code as *mut u32).cast()),
            &mut status_size,
            std::ptr::null_mut(),
        )
        .is_err()
        {
            return Err("failed to read HTTP status".to_owned());
        }
        if status_code == 404 {
            return Err("no published release yet".to_owned());
        }
        if !(200..300).contains(&status_code) {
            return Err(format!("HTTP {status_code}"));
        }

        let mut body = Vec::new();
        loop {
            let mut avail: u32 = 0;
            if WinHttpQueryDataAvailable(request, &mut avail).is_err() || avail == 0 {
                break;
            }
            let to_read = (avail as usize).min(16 * 1024);
            if body.len().saturating_add(to_read) > MAX_RESPONSE_BYTES {
                return Err("response too large".to_owned());
            }
            let mut chunk = vec![0u8; to_read];
            let mut read_bytes: u32 = 0;
            if WinHttpReadData(
                request,
                chunk.as_mut_ptr().cast(),
                to_read as u32,
                &mut read_bytes,
            )
            .is_err()
                || read_bytes == 0
            {
                break;
            }
            body.extend_from_slice(&chunk[..read_bytes as usize]);
        }

        String::from_utf8(body).map_err(|_| "invalid UTF-8 response".to_owned())
    }
}
