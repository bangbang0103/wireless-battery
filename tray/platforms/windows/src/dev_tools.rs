//! Development-only native Win32 floating debug panel & tray menu hooks.
//!
//! Compiled strictly behind `#[cfg(feature = "dev-tools")]` so release builds
//! contain zero dev UI or test actions.

use crate::i18n::Lang;
use crate::popover::PopoverWindow;
use crate::theme::SystemTheme;
use crate::tray::TrayIconHost;
use std::sync::{mpsc, Arc, Mutex};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, GetStockObject, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DEFAULT_PITCH, FF_SWISS, FW_NORMAL, HFONT, OUT_DEFAULT_PRECIS, WHITE_BRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyWindow,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, LoadCursorW, PostMessageW,
    RegisterClassExW, SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowTextW,
    ShowWindow, SystemParametersInfoW, CBS_DROPDOWNLIST, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL,
    ES_AUTOHSCROLL, ES_NUMBER, GWLP_USERDATA, HMENU, IDC_ARROW, MF_POPUP, MF_SEPARATOR, MF_STRING,
    SPI_GETWORKAREA, SW_HIDE, SW_SHOW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_SETFONT, WNDCLASSEXW, WS_BORDER, WS_CAPTION,
    WS_CHILD, WS_EX_TOPMOST, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};
use wireless_battery::{ChargeState, ErrorKind, Freshness, Reachability};
use wireless_battery_tray_core::dev_tools::DevDeviceSpec;
use wireless_battery_tray_core::{DeviceKind, PollerHandle};

/// Custom window message posted from the Dev panel to the main host message loop.
pub const WM_DEV_ACTION: u32 = WM_APP + 60;

pub const MENU_CMD_DEV_OPEN_PANEL: u32 = 2001;
pub const MENU_CMD_DEV_TOGGLE_LOW: u32 = 2002;
pub const MENU_CMD_DEV_NOTIFY_LOW_10: u32 = 2003;
pub const MENU_CMD_DEV_NOTIFY_LOW_5: u32 = 2004;
pub const MENU_CMD_DEV_ADD_PRESET_SUITE: u32 = 2005;
pub const MENU_CMD_DEV_RESET: u32 = 2006;

const CTRL_BTN_TOGGLE_LOW: isize = 3001;
const CTRL_BTN_NOTIFY_10: isize = 3002;
const CTRL_BTN_NOTIFY_5: isize = 3003;
const CTRL_EDIT_NAME: isize = 3010;
const CTRL_COMBO_KIND: isize = 3011;
const CTRL_EDIT_PCT: isize = 3012;
const CTRL_COMBO_STATE: isize = 3013;
const CTRL_BTN_ADD_DEVICE: isize = 3014;
const CTRL_BTN_PRESET_SUITE: isize = 3020;
const CTRL_BTN_RESET: isize = 3021;

#[derive(Debug, Clone)]
pub enum DevAction {
    ToggleLowBattery,
    SendLowBatteryNotification(u8),
    AddDevice(DevDeviceSpec),
    AddPresetSuite,
    ResetAll,
}

struct DevPanelInner {
    hwnd: HWND,
    font: HFONT,
    btn_toggle_low: HWND,
    edit_name: HWND,
    combo_kind: HWND,
    edit_pct: HWND,
    combo_state: HWND,
    status_label: HWND,
    host_hwnd: HWND,
    action_tx: Arc<Mutex<mpsc::Sender<DevAction>>>,
}

/// Independent floating Win32 Dev Tools window (compiled only with `--features dev-tools`).
pub struct DevPanelWindow {
    inner: Box<DevPanelInner>,
}

impl DevPanelWindow {
    pub fn create(
        host_hwnd: HWND,
        action_tx: Arc<Mutex<mpsc::Sender<DevAction>>>,
    ) -> windows::core::Result<Self> {
        let class_name = w!("WirelessBatteryTrayDevPanel");
        unsafe {
            let hinstance = GetModuleHandleW(None)?;
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(dev_panel_wnd_proc),
                hInstance: hinstance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(GetStockObject(WHITE_BRUSH).0),
                lpszClassName: class_name,
                ..Default::default()
            };
            let _ = RegisterClassExW(&wc);

            let width = 340;
            let height = 390;
            let (pos_x, pos_y) = default_panel_position(width, height);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST,
                class_name,
                w!("Wireless Battery — Dev 调试面板"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                pos_x,
                pos_y,
                width,
                height,
                None,
                None,
                hinstance,
                None,
            );
            if hwnd.0 == 0 {
                return Err(windows::core::Error::from_win32());
            }

            let font = CreateFontW(
                -14,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_SWISS.0) as u32,
                w!("Segoe UI"),
            );

            // Section 1: Low-battery state & notifications
            create_static(hwnd, font, "状态与通知调试", 14, 12, 296, 20);
            let btn_toggle_low = create_button(
                hwnd,
                font,
                "进入低电量状态",
                14,
                36,
                296,
                30,
                CTRL_BTN_TOGGLE_LOW,
            );
            create_button(
                hwnd,
                font,
                "发送低电量通知 (10%)",
                14,
                72,
                144,
                28,
                CTRL_BTN_NOTIFY_10,
            );
            create_button(
                hwnd,
                font,
                "严重低电通知 (5%)",
                166,
                72,
                144,
                28,
                CTRL_BTN_NOTIFY_5,
            );

            // Section 2: Manual mock device injection
            create_static(hwnd, font, "手动添加模拟设备", 14, 114, 296, 20);

            create_static(hwnd, font, "名称", 14, 140, 42, 22);
            let edit_name = create_edit(
                hwnd,
                font,
                "Dev Wireless Mouse",
                62,
                138,
                248,
                24,
                CTRL_EDIT_NAME,
                false,
            );

            create_static(hwnd, font, "类型", 14, 172, 42, 22);
            let combo_kind = create_combo(
                hwnd,
                font,
                &[
                    "Mouse (鼠标)",
                    "Keyboard (键盘)",
                    "Headset (耳机)",
                    "Gamepad (手柄)",
                ],
                0,
                62,
                168,
                130,
                140,
                CTRL_COMBO_KIND,
            );

            create_static(hwnd, font, "电量%", 200, 172, 44, 22);
            let edit_pct = create_edit(hwnd, font, "85", 248, 168, 62, 24, CTRL_EDIT_PCT, true);

            create_static(hwnd, font, "状态", 14, 204, 42, 22);
            let combo_state = create_combo(
                hwnd,
                font,
                &[
                    "正常放电 (Live)",
                    "充电中 (Charging)",
                    "休眠缓存 (Stale)",
                    "读取失败 (Error)",
                ],
                0,
                62,
                200,
                248,
                140,
                CTRL_COMBO_STATE,
            );

            create_button(
                hwnd,
                font,
                "+ 添加设备到列表",
                14,
                234,
                296,
                30,
                CTRL_BTN_ADD_DEVICE,
            );

            // Section 3: Presets & reset
            create_button(
                hwnd,
                font,
                "填充多设备测试场景",
                14,
                274,
                156,
                28,
                CTRL_BTN_PRESET_SUITE,
            );
            create_button(hwnd, font, "清空 / 重置", 178, 274, 132, 28, CTRL_BTN_RESET);

            let status_label = create_static(
                hwnd,
                font,
                "Dev 模式已就绪（关闭窗口仅隐藏，托盘右键可重开）",
                14,
                314,
                296,
                34,
            );

            let mut inner = Box::new(DevPanelInner {
                hwnd,
                font,
                btn_toggle_low,
                edit_name,
                combo_kind,
                edit_pct,
                combo_state,
                status_label,
                host_hwnd,
                action_tx,
            });

            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                (&mut *inner as *mut DevPanelInner) as isize,
            );
            let _ = ShowWindow(hwnd, SW_SHOW);

            Ok(Self { inner })
        }
    }

    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.inner.hwnd, SW_SHOW);
            let _ = SetForegroundWindow(self.inner.hwnd);
        }
    }

    pub fn set_low_battery_forced(&self, forced: bool) {
        let label = if forced {
            "退出低电量状态 (当前: 低电量)"
        } else {
            "进入低电量状态"
        };
        unsafe {
            let _ = SetWindowTextW(self.inner.btn_toggle_low, &HSTRING::from(label));
        }
    }

    pub fn set_status(&self, text: &str) {
        unsafe {
            let _ = SetWindowTextW(self.inner.status_label, &HSTRING::from(text));
        }
    }
}

impl Drop for DevPanelWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = SetWindowLongPtrW(self.inner.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.inner.hwnd);
            if self.inner.font.0 != 0 {
                let _ = DeleteObject(self.inner.font);
            }
        }
    }
}

/// Append the `Dev Tools` submenu to the tray right-click menu.
pub unsafe fn append_context_menu(menu: HMENU) {
    if let Ok(dev_menu) = CreatePopupMenu() {
        let _ = AppendMenuW(
            dev_menu,
            MF_STRING,
            MENU_CMD_DEV_OPEN_PANEL as usize,
            w!("打开 Dev 调试面板"),
        );
        let _ = AppendMenuW(dev_menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(
            dev_menu,
            MF_STRING,
            MENU_CMD_DEV_TOGGLE_LOW as usize,
            w!("切换低电量状态"),
        );
        let _ = AppendMenuW(
            dev_menu,
            MF_STRING,
            MENU_CMD_DEV_NOTIFY_LOW_10 as usize,
            w!("发送低电量通知 (10%)"),
        );
        let _ = AppendMenuW(
            dev_menu,
            MF_STRING,
            MENU_CMD_DEV_NOTIFY_LOW_5 as usize,
            w!("发送严重低电通知 (5%)"),
        );
        let _ = AppendMenuW(
            dev_menu,
            MF_STRING,
            MENU_CMD_DEV_ADD_PRESET_SUITE as usize,
            w!("填充多设备测试场景"),
        );
        let _ = AppendMenuW(dev_menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(
            dev_menu,
            MF_STRING,
            MENU_CMD_DEV_RESET as usize,
            w!("清空虚拟设备 / 重置"),
        );
        let _ = AppendMenuW(menu, MF_POPUP, dev_menu.0 as usize, w!("Dev 调试"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    }
}

/// Handle a Dev submenu command selected from the tray context menu.
pub fn handle_context_menu_cmd(
    cmd: u32,
    dev_panel: &DevPanelWindow,
    poller: &PollerHandle,
    tray: &mut TrayIconHost,
    popover: &mut PopoverWindow,
    theme: SystemTheme,
) {
    match cmd {
        MENU_CMD_DEV_OPEN_PANEL => dev_panel.show(),
        MENU_CMD_DEV_TOGGLE_LOW => dispatch_dev_action(
            DevAction::ToggleLowBattery,
            dev_panel,
            poller,
            tray,
            popover,
            theme,
        ),
        MENU_CMD_DEV_NOTIFY_LOW_10 => dispatch_dev_action(
            DevAction::SendLowBatteryNotification(10),
            dev_panel,
            poller,
            tray,
            popover,
            theme,
        ),
        MENU_CMD_DEV_NOTIFY_LOW_5 => dispatch_dev_action(
            DevAction::SendLowBatteryNotification(5),
            dev_panel,
            poller,
            tray,
            popover,
            theme,
        ),
        MENU_CMD_DEV_ADD_PRESET_SUITE => dispatch_dev_action(
            DevAction::AddPresetSuite,
            dev_panel,
            poller,
            tray,
            popover,
            theme,
        ),
        MENU_CMD_DEV_RESET => {
            dispatch_dev_action(DevAction::ResetAll, dev_panel, poller, tray, popover, theme)
        }
        _ => {}
    }
}

/// Execute a `DevAction` against `TrayService` and push the updated state to tray + popover.
pub fn dispatch_dev_action(
    action: DevAction,
    dev_panel: &DevPanelWindow,
    poller: &PollerHandle,
    tray: &mut TrayIconHost,
    popover: &mut PopoverWindow,
    theme: SystemTheme,
) {
    let service = poller.service();
    match action {
        DevAction::ToggleLowBattery => {
            let (forced, snap) = service.dev_toggle_low_battery();
            dev_panel.set_low_battery_forced(forced);
            dev_panel.set_status(if forced {
                "已进入低电量状态（托盘波纹已变红）"
            } else {
                "已退出强制低电量状态"
            });
            tray.update(snap.icon_state, theme.taskbar_dark, &snap.tooltip);
            popover.set_snapshot(snap);
        }
        DevAction::SendLowBatteryNotification(pct) => {
            let alert = service.dev_make_low_battery_alert(pct);
            let lang = Lang::resolve(service.config().language);
            tray.notify_low_battery(&alert, lang);
            dev_panel.set_status(&format!(
                "已发送系统低电量通知：{} ({}%)",
                alert.device_name, alert.percentage
            ));
        }
        DevAction::AddDevice(spec) => {
            let name = spec.name.clone();
            let pct_text = spec
                .percentage
                .map_or_else(|| "Error".to_owned(), |p| format!("{p}%"));
            let snap = service.dev_add_device(spec);
            tray.update(snap.icon_state, theme.taskbar_dark, &snap.tooltip);
            popover.set_snapshot(snap);
            dev_panel.set_status(&format!("已添加模拟设备：{name} ({pct_text})"));
        }
        DevAction::AddPresetSuite => {
            let presets = [
                DevDeviceSpec {
                    name: "Dev Pro Mouse".to_owned(),
                    kind: DeviceKind::Mouse,
                    percentage: Some(88),
                    charge_state: ChargeState::Discharging,
                    reachability: Reachability::Online,
                    freshness: Some(Freshness::Live),
                    last_error: None,
                },
                DevDeviceSpec {
                    name: "Dev Mechanical Keyboard".to_owned(),
                    kind: DeviceKind::Keyboard,
                    percentage: Some(45),
                    charge_state: ChargeState::Charging,
                    reachability: Reachability::Online,
                    freshness: Some(Freshness::Live),
                    last_error: None,
                },
                DevDeviceSpec {
                    name: "Dev Wireless Headset".to_owned(),
                    kind: DeviceKind::Headset,
                    percentage: Some(12),
                    charge_state: ChargeState::Discharging,
                    reachability: Reachability::Online,
                    freshness: Some(Freshness::Live),
                    last_error: None,
                },
                DevDeviceSpec {
                    name: "Dev Gamepad".to_owned(),
                    kind: DeviceKind::Gamepad,
                    percentage: Some(60),
                    charge_state: ChargeState::Discharging,
                    reachability: Reachability::Sleeping,
                    freshness: Some(Freshness::Stale),
                    last_error: None,
                },
            ];
            let mut snap = service.snapshot();
            for spec in presets {
                snap = service.dev_add_device(spec);
            }
            tray.update(snap.icon_state, theme.taskbar_dark, &snap.tooltip);
            popover.set_snapshot(snap);
            dev_panel.set_status("已填充 4 台多状态测试设备（高电/充电/低电/休眠）");
        }
        DevAction::ResetAll => {
            let snap = service.dev_reset();
            dev_panel.set_low_battery_forced(false);
            tray.update(snap.icon_state, theme.taskbar_dark, &snap.tooltip);
            popover.set_snapshot(snap);
            dev_panel.set_status("已清空虚拟设备并恢复真实状态");
        }
    }
}

unsafe extern "system" fn dev_panel_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DevPanelInner;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let inner = &mut *ptr;

    match msg {
        WM_CLOSE => {
            // Hide instead of quitting the tray app so the user can reopen from tray menu.
            let _ = ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }
        WM_COMMAND => {
            let ctrl_id = (wparam.0 & 0xFFFF) as isize;
            let notify_code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            if notify_code == 0 {
                // BN_CLICKED
                let action = match ctrl_id {
                    CTRL_BTN_TOGGLE_LOW => Some(DevAction::ToggleLowBattery),
                    CTRL_BTN_NOTIFY_10 => Some(DevAction::SendLowBatteryNotification(10)),
                    CTRL_BTN_NOTIFY_5 => Some(DevAction::SendLowBatteryNotification(5)),
                    CTRL_BTN_ADD_DEVICE => Some(DevAction::AddDevice(read_device_form(inner))),
                    CTRL_BTN_PRESET_SUITE => Some(DevAction::AddPresetSuite),
                    CTRL_BTN_RESET => Some(DevAction::ResetAll),
                    _ => None,
                };
                if let Some(act) = action {
                    if let Ok(guard) = inner.action_tx.lock() {
                        let _ = guard.send(act);
                    }
                    let _ = PostMessageW(inner.host_hwnd, WM_DEV_ACTION, WPARAM(0), LPARAM(0));
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn read_device_form(inner: &DevPanelInner) -> DevDeviceSpec {
    let raw_name = get_window_text(inner.edit_name);
    let name = if raw_name.trim().is_empty() {
        "Dev Mock Device".to_owned()
    } else {
        raw_name.trim().to_owned()
    };

    let kind_idx = SendMessageW(inner.combo_kind, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
    let kind = match kind_idx {
        1 => DeviceKind::Keyboard,
        2 => DeviceKind::Headset,
        3 => DeviceKind::Gamepad,
        _ => DeviceKind::Mouse,
    };

    let pct_val = get_window_text(inner.edit_pct)
        .trim()
        .parse::<u8>()
        .unwrap_or(50)
        .min(100);

    let state_idx = SendMessageW(inner.combo_state, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
    match state_idx {
        1 => DevDeviceSpec {
            name,
            kind,
            percentage: Some(pct_val),
            charge_state: ChargeState::Charging,
            reachability: Reachability::Online,
            freshness: Some(Freshness::Live),
            last_error: None,
        },
        2 => DevDeviceSpec {
            name,
            kind,
            percentage: Some(pct_val),
            charge_state: ChargeState::Discharging,
            reachability: Reachability::Sleeping,
            freshness: Some(Freshness::Stale),
            last_error: None,
        },
        3 => DevDeviceSpec {
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
            percentage: Some(pct_val),
            charge_state: ChargeState::Discharging,
            reachability: Reachability::Online,
            freshness: Some(Freshness::Live),
            last_error: None,
        },
    }
}

unsafe fn get_window_text(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let copied = GetWindowTextW(hwnd, &mut buf);
    String::from_utf16_lossy(&buf[..copied as usize])
}

unsafe fn default_panel_position(width: i32, height: i32) -> (i32, i32) {
    let mut work = RECT::default();
    if SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        Some(&mut work as *mut RECT as *mut std::ffi::c_void),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    )
    .is_ok()
    {
        (
            (work.right - width - 380).max(20),
            (work.bottom - height - 24).max(20),
        )
    } else {
        (120, 120)
    }
}

unsafe fn apply_font(hwnd: HWND, font: HFONT) {
    if font.0 != 0 {
        let _ = SendMessageW(hwnd, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
    }
}

unsafe fn create_static(
    parent: HWND,
    font: HFONT,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> HWND {
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("STATIC"),
        &HSTRING::from(text),
        WS_CHILD | WS_VISIBLE,
        x,
        y,
        w,
        h,
        parent,
        None,
        None,
        None,
    );
    apply_font(hwnd, font);
    hwnd
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_button(
    parent: HWND,
    font: HFONT,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: isize,
) -> HWND {
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        &HSTRING::from(text),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        x,
        y,
        w,
        h,
        parent,
        HMENU(id),
        None,
        None,
    );
    apply_font(hwnd, font);
    hwnd
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_edit(
    parent: HWND,
    font: HFONT,
    initial: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: isize,
    numeric: bool,
) -> HWND {
    let mut style =
        WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32);
    if numeric {
        style |= WINDOW_STYLE(ES_NUMBER as u32);
    }
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("EDIT"),
        &HSTRING::from(initial),
        style,
        x,
        y,
        w,
        h,
        parent,
        HMENU(id),
        None,
        None,
    );
    apply_font(hwnd, font);
    hwnd
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_combo(
    parent: HWND,
    font: HFONT,
    items: &[&str],
    selected: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: isize,
) -> HWND {
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("COMBOBOX"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        x,
        y,
        w,
        h,
        parent,
        HMENU(id),
        None,
        None,
    );
    apply_font(hwnd, font);
    for item in items {
        let wide = HSTRING::from(*item);
        let _ = SendMessageW(
            hwnd,
            CB_ADDSTRING,
            WPARAM(0),
            LPARAM(wide.as_ptr() as isize),
        );
    }
    let _ = SendMessageW(hwnd, CB_SETCURSEL, WPARAM(selected), LPARAM(0));
    hwnd
}
