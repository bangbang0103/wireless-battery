//! Minimal Windows HID calls missing from HIDAPI 2.6.
#![allow(unsafe_code)]

use super::TransportError;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_READ_WRITE: u32 = 0x0000_0003;
const OPEN_EXISTING: u32 = 3;
const INVALID_HANDLE: isize = -1;

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *mut c_void,
        creation: u32,
        flags: u32,
        template: isize,
    ) -> isize;
    fn CloseHandle(handle: isize) -> i32;
}

#[link(name = "hid")]
extern "system" {
    fn HidD_SetOutputReport(handle: isize, report: *mut c_void, length: u32) -> i32;
    fn HidD_GetInputReport(handle: isize, report: *mut c_void, length: u32) -> i32;
}

pub(crate) struct WindowsHid {
    handle: isize,
}

impl WindowsHid {
    pub(crate) fn open(path: &str) -> Result<Self, TransportError> {
        let wide = std::ffi::OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let mut last_error = None;
        for access in [GENERIC_READ | GENERIC_WRITE, GENERIC_WRITE] {
            // SAFETY: the path is NUL-terminated and all optional pointers are null.
            let handle = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    access,
                    FILE_SHARE_READ_WRITE,
                    std::ptr::null_mut(),
                    OPEN_EXISTING,
                    0,
                    0,
                )
            };
            if handle != INVALID_HANDLE {
                return Ok(Self { handle });
            }
            last_error = Some(std::io::Error::last_os_error().to_string());
        }
        Err(super::classify_hid_error(
            last_error.unwrap_or_else(|| "CreateFileW failed".to_owned()),
        ))
    }

    pub(crate) fn set_output(&self, report: &mut [u8]) -> Result<(), TransportError> {
        let length = u32::try_from(report.len()).map_err(|_| {
            TransportError::InvalidResponse("output report exceeds Win32 length".to_owned())
        })?;
        // SAFETY: the handle is owned by self and the mutable report buffer is valid
        // for exactly `length` bytes for the duration of this synchronous call.
        let ok = unsafe {
            HidD_SetOutputReport(self.handle, report.as_mut_ptr().cast::<c_void>(), length)
        };
        if ok == 0 {
            Err(super::classify_hid_error(
                std::io::Error::last_os_error().to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn get_input(&self, report: &mut [u8]) -> Result<(), TransportError> {
        let length = u32::try_from(report.len()).map_err(|_| {
            TransportError::InvalidResponse("input report exceeds Win32 length".to_owned())
        })?;
        // SAFETY: the handle is owned by self and the mutable report buffer is valid
        // for exactly `length` bytes for the duration of this synchronous call.
        let ok = unsafe {
            HidD_GetInputReport(self.handle, report.as_mut_ptr().cast::<c_void>(), length)
        };
        if ok == 0 {
            Err(super::classify_hid_error(
                std::io::Error::last_os_error().to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

impl Drop for WindowsHid {
    fn drop(&mut self) {
        // SAFETY: self uniquely owns this valid handle and drop runs once.
        unsafe {
            CloseHandle(self.handle);
        }
    }
}
