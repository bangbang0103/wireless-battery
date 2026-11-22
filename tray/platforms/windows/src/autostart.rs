//! Windows autostart registry integration and UTF-16 clipboard helper.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_SZ,
};

const RUN_SUBKEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const RUN_VALUE_NAME: PCWSTR = w!("WirelessBatteryTray");
const CF_UNICODETEXT: u32 = 13;

/// Enable or disable silent launch on Windows sign-in via `HKCU\...\Run`.
pub fn set_autostart(enabled: bool) -> bool {
    let mut hkey = HKEY::default();
    let status =
        unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_SUBKEY, 0, KEY_SET_VALUE, &mut hkey) };
    if status.is_err() {
        return false;
    }

    let ok = if enabled {
        if let Ok(exe_path) = std::env::current_exe() {
            let quoted = format!("\"{}\"", exe_path.display());
            let wide: Vec<u16> = OsStr::new(&quoted)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let bytes: &[u8] =
                unsafe { std::slice::from_raw_parts(wide.as_ptr().cast(), wide.len() * 2) };
            unsafe { RegSetValueExW(hkey, RUN_VALUE_NAME, 0, REG_SZ, Some(bytes)).is_ok() }
        } else {
            false
        }
    } else {
        // Treat missing value as already disabled.
        let _ = unsafe { RegDeleteValueW(hkey, RUN_VALUE_NAME) };
        true
    };

    unsafe {
        let _ = RegCloseKey(hkey);
    }
    ok
}

/// Copy a UTF-8 string onto the Windows clipboard as `CF_UNICODETEXT`.
pub fn copy_text_to_clipboard(owner: HWND, text: &str) -> bool {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = wide.len() * std::mem::size_of::<u16>();

    unsafe {
        if OpenClipboard(owner).is_err() {
            return false;
        }
        let _ = EmptyClipboard();

        let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, byte_len) else {
            let _ = CloseClipboard();
            return false;
        };

        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            let _ = GlobalFree(hmem);
            let _ = CloseClipboard();
            return false;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr().cast::<u8>(), ptr.cast::<u8>(), byte_len);
        let _ = GlobalUnlock(hmem);

        let result = SetClipboardData(CF_UNICODETEXT, HANDLE(hmem.0 as isize));
        if result.is_err() {
            let _ = GlobalFree(hmem);
        }
        let _ = CloseClipboard();
        result.is_ok()
    }
}
