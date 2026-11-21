//! Windows system tray (`Shell_NotifyIconW`) integration: pure Logo icon,
//! multi-line hover tooltip, low-battery native notifications, and context menu.

use crate::icon::{build_tray_logo_icon, OwnedIcon};
use windows::core::w;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconGetRect, Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP,
    NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NOTIFYICONDATAW,
    NOTIFYICONIDENTIFIER, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow, TrackPopupMenu,
    MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_NONOTIFY, TPM_RETURNCMD, WM_APP,
};
use wireless_battery_tray_core::{LowBatteryAlert, TrayIconState};

/// Custom window message delivered when the user interacts with the tray icon.
pub const WM_TRAY_CALLBACK: u32 = WM_APP + 10;

pub const MENU_CMD_OPEN_PANEL: u32 = 1001;
pub const MENU_CMD_TOGGLE_AUTOSTART: u32 = 1002;
pub const MENU_CMD_QUIT: u32 = 1003;

const TRAY_ICON_UID: u32 = 1;

pub struct TrayIconHost {
    owner: HWND,
    current_icon: Option<OwnedIcon>,
    icon_state: TrayIconState,
    taskbar_dark: bool,
    tooltip: String,
    added: bool,
}

impl TrayIconHost {
    pub fn new(owner: HWND, taskbar_dark: bool) -> Self {
        Self {
            owner,
            current_icon: None,
            icon_state: TrayIconState::Idle,
            taskbar_dark,
            tooltip: "Wireless Battery".to_owned(),
            added: false,
        }
    }

    /// Add (or re-add after Explorer restart) the pure-Logo icon to the taskbar notification area.
    pub fn install(&mut self) -> bool {
        self.current_icon = build_tray_logo_icon(self.icon_state, self.taskbar_dark);

        let mut nid = self.base_nid();
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY_CALLBACK;
        if let Some(icon) = &self.current_icon {
            nid.hIcon = icon.handle();
        }
        write_wide_slice(&mut nid.szTip, &self.tooltip);

        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
        if ok {
            nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            unsafe {
                let _ = Shell_NotifyIconW(NIM_SETVERSION, &nid);
            }
            self.added = true;
        }
        ok
    }

    /// Update tray logo appearance and hover tooltip when snapshot or system theme changes.
    pub fn update(&mut self, icon_state: TrayIconState, taskbar_dark: bool, tooltip: &str) {
        let icon_changed = self.icon_state != icon_state
            || self.taskbar_dark != taskbar_dark
            || self.current_icon.is_none();
        let tip_changed = self.tooltip != tooltip;

        self.icon_state = icon_state;
        self.taskbar_dark = taskbar_dark;
        self.tooltip = tooltip.to_owned();

        if !self.added {
            let _ = self.install();
            return;
        }

        if !icon_changed && !tip_changed {
            return;
        }

        let mut nid = self.base_nid();
        if icon_changed {
            self.current_icon = build_tray_logo_icon(icon_state, taskbar_dark);
            if let Some(icon) = &self.current_icon {
                nid.hIcon = icon.handle();
                nid.uFlags |= NIF_ICON;
            }
        }
        if tip_changed {
            write_wide_slice(&mut nid.szTip, tooltip);
            nid.uFlags |= NIF_TIP;
        }

        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    /// Emit a native Windows Action Center notification for a low-battery peripheral.
    pub fn notify_low_battery(&self, alert: &LowBatteryAlert) {
        if !self.added {
            return;
        }
        let mut nid = self.base_nid();
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = NIIF_WARNING;
        write_wide_slice(&mut nid.szInfoTitle, "外设低电量提醒");
        let body = format!(
            "{} 仅剩 {}%，建议连接充电。",
            alert.device_name, alert.percentage
        );
        write_wide_slice(&mut nid.szInfo, &body);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    /// Query the tray icon's screen bounding rectangle for precise popover anchoring.
    pub fn anchor_rect(&self) -> Option<RECT> {
        let ident = NOTIFYICONIDENTIFIER {
            cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
            hWnd: self.owner,
            uID: TRAY_ICON_UID,
            ..Default::default()
        };
        unsafe { Shell_NotifyIconGetRect(&ident).ok() }
    }

    /// Show a native Win32 context menu on tray right-click: Open Panel, Autostart (checked), Quit.
    pub fn show_context_menu(&self, autostart_enabled: bool) -> Option<u32> {
        unsafe {
            let menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_CMD_OPEN_PANEL as usize,
                w!("打开面板"),
            );
            let autostart_flags = MF_STRING
                | if autostart_enabled {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                };
            let _ = AppendMenuW(
                menu,
                autostart_flags,
                MENU_CMD_TOGGLE_AUTOSTART as usize,
                w!("开机自启"),
            );
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(menu, MF_STRING, MENU_CMD_QUIT as usize, w!("退出"));

            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let _ = SetForegroundWindow(self.owner);

            let cmd = TrackPopupMenu(
                menu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RETURNCMD | TPM_NONOTIFY,
                pt.x,
                pt.y,
                0,
                self.owner,
                None,
            );
            let _ = DestroyMenu(menu);
            if cmd.0 > 0 {
                Some(cmd.0 as u32)
            } else {
                None
            }
        }
    }

    /// Remove the icon cleanly before process exit.
    pub fn remove(&mut self) {
        if self.added {
            let nid = self.base_nid();
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
            self.added = false;
        }
    }

    fn base_nid(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.owner,
            uID: TRAY_ICON_UID,
            ..Default::default()
        }
    }
}

impl Drop for TrayIconHost {
    fn drop(&mut self) {
        self.remove();
    }
}

fn write_wide_slice(dest: &mut [u16], text: &str) {
    dest.fill(0);
    if dest.is_empty() {
        return;
    }
    let max_chars = dest.len() - 1;
    for (slot, ch) in dest.iter_mut().take(max_chars).zip(text.encode_utf16()) {
        *slot = ch;
    }
}
