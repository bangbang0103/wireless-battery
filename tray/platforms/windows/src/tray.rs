//! Windows system tray (`Shell_NotifyIconW`) integration: pure Logo icon,
//! multi-line hover tooltip, low-battery native notifications, and bilingual context menu.

use crate::i18n::Lang;
use crate::icon::{build_tray_logo_icon, OwnedIcon};
use windows::core::HSTRING;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconGetRect, Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP,
    NIIF_ERROR, NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NOTIFYICONDATAW,
    NOTIFYICONIDENTIFIER, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuRadioItem, CreatePopupMenu, DestroyMenu, GetCursorPos,
    SetForegroundWindow, TrackPopupMenu, MF_BYCOMMAND, MF_CHECKED, MF_GRAYED, MF_POPUP,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY,
    TPM_RETURNCMD, WM_APP,
};
use wireless_battery_tray_core::{LanguageMode, LowBatteryAlert, ThemeMode, TrayIconState};

/// Custom window message delivered when the user interacts with the tray icon.
pub const WM_TRAY_CALLBACK: u32 = WM_APP + 10;

pub const MENU_CMD_OPEN_PANEL: u32 = 1001;
pub const MENU_CMD_TOGGLE_AUTOSTART: u32 = 1002;
pub const MENU_CMD_QUIT: u32 = 1003;
pub const MENU_CMD_TOGGLE_LOW_BATTERY_NOTIFY: u32 = 1004;
pub const MENU_CMD_TOGGLE_LOW_BATTERY_SILENT: u32 = 1005;
pub const MENU_CMD_THEME_SYSTEM: u32 = 1010;
pub const MENU_CMD_THEME_LIGHT: u32 = 1011;
pub const MENU_CMD_THEME_DARK: u32 = 1012;
pub const MENU_CMD_LANG_SYSTEM: u32 = 1020;
pub const MENU_CMD_LANG_ZH_CN: u32 = 1021;
pub const MENU_CMD_LANG_EN: u32 = 1022;

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

    /// Emit a native Windows Action Center notification for a low-battery peripheral
    /// (tiered at 15%, 10%, 5%, 1%; uses critical error icon at ≤ 5%).
    pub fn notify_low_battery(&self, alert: &LowBatteryAlert, lang: Lang) {
        if !self.added {
            return;
        }
        let mut nid = self.base_nid();
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = if alert.percentage <= 5 {
            NIIF_ERROR
        } else {
            NIIF_WARNING
        };
        write_wide_slice(&mut nid.szInfoTitle, lang.notify_title(alert.percentage));
        let body = lang.notify_body(&alert.device_name, alert.percentage);
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

    /// Show a bilingual native Win32 context menu on tray right-click.
    #[allow(clippy::too_many_arguments)]
    pub fn show_context_menu(
        &self,
        autostart_enabled: bool,
        low_battery_notify: bool,
        low_battery_silent: bool,
        theme_mode: ThemeMode,
        language_mode: LanguageMode,
        lang: Lang,
    ) -> Option<u32> {
        unsafe {
            let menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_CMD_OPEN_PANEL as usize,
                &HSTRING::from(lang.menu_open_panel()),
            );

            if let Ok(theme_menu) = CreatePopupMenu() {
                let _ = AppendMenuW(
                    theme_menu,
                    MF_STRING,
                    MENU_CMD_THEME_SYSTEM as usize,
                    &HSTRING::from(lang.theme_mode_label(ThemeMode::System)),
                );
                let _ = AppendMenuW(
                    theme_menu,
                    MF_STRING,
                    MENU_CMD_THEME_LIGHT as usize,
                    &HSTRING::from(lang.menu_theme_light()),
                );
                let _ = AppendMenuW(
                    theme_menu,
                    MF_STRING,
                    MENU_CMD_THEME_DARK as usize,
                    &HSTRING::from(lang.menu_theme_dark()),
                );
                let checked_cmd = match theme_mode {
                    ThemeMode::System => MENU_CMD_THEME_SYSTEM,
                    ThemeMode::Light => MENU_CMD_THEME_LIGHT,
                    ThemeMode::Dark => MENU_CMD_THEME_DARK,
                };
                let _ = CheckMenuRadioItem(
                    theme_menu,
                    MENU_CMD_THEME_SYSTEM,
                    MENU_CMD_THEME_DARK,
                    checked_cmd,
                    MF_BYCOMMAND.0,
                );
                let _ = AppendMenuW(
                    menu,
                    MF_POPUP,
                    theme_menu.0 as usize,
                    &HSTRING::from(lang.menu_theme()),
                );
            }

            if let Ok(lang_menu) = CreatePopupMenu() {
                let _ = AppendMenuW(
                    lang_menu,
                    MF_STRING,
                    MENU_CMD_LANG_SYSTEM as usize,
                    &HSTRING::from(lang.language_mode_label(LanguageMode::System)),
                );
                let _ = AppendMenuW(
                    lang_menu,
                    MF_STRING,
                    MENU_CMD_LANG_ZH_CN as usize,
                    &HSTRING::from(lang.language_mode_label(LanguageMode::ZhCn)),
                );
                let _ = AppendMenuW(
                    lang_menu,
                    MF_STRING,
                    MENU_CMD_LANG_EN as usize,
                    &HSTRING::from(lang.language_mode_label(LanguageMode::En)),
                );
                let checked_cmd = match language_mode {
                    LanguageMode::System => MENU_CMD_LANG_SYSTEM,
                    LanguageMode::ZhCn => MENU_CMD_LANG_ZH_CN,
                    LanguageMode::En => MENU_CMD_LANG_EN,
                };
                let _ = CheckMenuRadioItem(
                    lang_menu,
                    MENU_CMD_LANG_SYSTEM,
                    MENU_CMD_LANG_EN,
                    checked_cmd,
                    MF_BYCOMMAND.0,
                );
                let _ = AppendMenuW(
                    menu,
                    MF_POPUP,
                    lang_menu.0 as usize,
                    &HSTRING::from(lang.menu_language()),
                );
            }

            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let notify_flags = MF_STRING
                | if low_battery_notify {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                };
            let _ = AppendMenuW(
                menu,
                notify_flags,
                MENU_CMD_TOGGLE_LOW_BATTERY_NOTIFY as usize,
                &HSTRING::from(lang.menu_low_battery()),
            );

            let mut silent_flags = MF_STRING
                | if low_battery_silent {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                };
            if !low_battery_notify {
                silent_flags |= MF_GRAYED;
            }
            let _ = AppendMenuW(
                menu,
                silent_flags,
                MENU_CMD_TOGGLE_LOW_BATTERY_SILENT as usize,
                &HSTRING::from(lang.menu_silent_notify()),
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
                &HSTRING::from(lang.menu_autostart()),
            );
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            #[cfg(feature = "dev-tools")]
            crate::dev_tools::append_context_menu(menu);
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_CMD_QUIT as usize,
                &HSTRING::from(lang.quit()),
            );

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
