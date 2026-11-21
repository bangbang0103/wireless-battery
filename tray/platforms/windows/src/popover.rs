//! Borderless Win11 Fluent Acrylic popover window anchored to the system tray.

use crate::icon::load_app_icon;
use crate::renderer::{
    HitTarget, PaintSurface, PopoverRenderer, PopoverView, UiState, HEADER_HEIGHT,
    POPOVER_WIDTH_DIP,
};
use crate::theme::SystemTheme;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetMonitorInfoW, InvalidateRect, MonitorFromPoint, UpdateWindow,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetClientRect, GetCursorPos, GetWindowLongPtrW,
    IsWindowVisible, KillTimer, LoadCursorW, RegisterClassExW, SendMessageW, SetCursor,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWLP_USERDATA,
    HTCAPTION, HWND_TOPMOST, IDC_ARROW, IDC_HAND, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    SW_SHOW, WA_INACTIVE, WM_ACTIVATE, WM_DPICHANGED, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCLBUTTONDOWN, WM_PAINT, WM_TIMER, WNDCLASSEXW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use wireless_battery_tray_core::TraySnapshot;

const WM_MOUSELEAVE: u32 = 0x02A3;

/// `DWMSBT_TRANSIENTWINDOW` (Acrylic backdrop designed for Win11 flyouts/popovers).
const DWMSBT_TRANSIENTWINDOW: u32 = 3;
const FEEDBACK_TIMER_ID: usize = 7001;
const COLLAPSE_TIMER_ID: usize = 7002;

/// User actions emitted from the popover back to the application controller.
#[derive(Debug, Clone)]
pub enum PopoverAction {
    RefreshAll,
    RefreshDevice(String),
    CopyDiagnostic(String),
    SetPollInterval(u64),
    ToggleLowBatteryNotify,
    ToggleAutostart,
    Quit,
}

type ActionCallback = Box<dyn FnMut(PopoverAction)>;

struct PopoverInner {
    hwnd: HWND,
    renderer: PopoverRenderer,
    ui: UiState,
    snapshot: TraySnapshot,
    theme: SystemTheme,
    acrylic_active: bool,
    compact_active: bool,
    tracking_mouse: bool,
    last_auto_hide: Option<Instant>,
    on_action: ActionCallback,
}

/// Owning handle for the native Direct2D + DWM Acrylic popover window.
pub struct PopoverWindow {
    inner: Box<PopoverInner>,
}

impl PopoverWindow {
    pub fn create(
        initial_snapshot: TraySnapshot,
        theme: SystemTheme,
        on_action: impl FnMut(PopoverAction) + 'static,
    ) -> windows::core::Result<Self> {
        let renderer = PopoverRenderer::new()?;
        let class_name = w!("WirelessBatteryTrayPopover");

        unsafe {
            let hinstance = GetModuleHandleW(None)?;
            let app_icon = load_app_icon(hinstance);
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(popover_wnd_proc),
                hInstance: hinstance.into(),
                hIcon: app_icon,
                hIconSm: app_icon,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                lpszClassName: class_name,
                ..Default::default()
            };
            let _ = RegisterClassExW(&wc);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name,
                w!("Wireless Battery"),
                WS_POPUP,
                0,
                0,
                344,
                260,
                None,
                None,
                hinstance,
                None,
            );
            if hwnd.0 == 0 {
                return Err(windows::core::Error::from_win32());
            }

            let acrylic_active = apply_dwm_surface(hwnd, theme.app_dark);

            let mut inner = Box::new(PopoverInner {
                hwnd,
                renderer,
                ui: UiState::default(),
                snapshot: initial_snapshot,
                theme,
                acrylic_active,
                compact_active: false,
                tracking_mouse: false,
                last_auto_hide: None,
                on_action: Box::new(on_action),
            });

            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                (&mut *inner as *mut PopoverInner) as isize,
            );

            // Pre-warm Direct2D render target and DirectWrite font caches while hidden so the
            // very first click renders immediately with zero white flash.
            paint_inner(&mut inner);

            Ok(Self { inner })
        }
    }

    pub fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.inner.hwnd).as_bool() }
    }

    /// Toggle visibility cleanly, debouncing click-on-tray-icon right after focus-loss hide.
    pub fn toggle(&mut self, anchor: Option<RECT>) {
        if self.is_visible() {
            self.hide();
            return;
        }
        if let Some(last) = self.inner.last_auto_hide {
            if last.elapsed().as_millis() < 220 {
                self.inner.last_auto_hide = None;
                return;
            }
        }
        self.show(anchor);
    }

    pub fn show(&mut self, anchor: Option<RECT>) {
        self.inner.ui.view = PopoverView::Main;
        self.inner.ui.hovered = None;
        self.inner.ui.mouse_inside = false;
        self.inner.compact_active = false;
        // Position and render synchronously while still hidden to eliminate any white flash.
        self.position_and_resize(anchor, true);
        paint_inner(&mut self.inner);
        unsafe {
            let _ = ShowWindow(self.inner.hwnd, SW_SHOW);
            let _ = SetForegroundWindow(self.inner.hwnd);
            let _ = UpdateWindow(self.inner.hwnd);
        }
    }

    pub fn hide(&mut self) {
        unsafe {
            let _ = KillTimer(self.inner.hwnd, COLLAPSE_TIMER_ID);
            let _ = ShowWindow(self.inner.hwnd, SW_HIDE);
        }
        self.inner.ui.pinned = false;
        self.inner.ui.mouse_inside = false;
        self.inner.compact_active = false;
        self.inner.ui.hovered = None;
        self.inner.ui.feedback_banner = None;
    }

    pub fn set_snapshot(&mut self, snapshot: TraySnapshot) {
        self.inner.snapshot = snapshot;
        if self.is_visible() {
            self.position_and_resize(None, false);
            unsafe {
                let _ = InvalidateRect(self.inner.hwnd, None, false);
            }
        }
    }

    pub fn set_theme(&mut self, theme: SystemTheme) {
        if self.inner.theme == theme {
            return;
        }
        self.inner.theme = theme;
        self.inner.acrylic_active = apply_dwm_surface(self.inner.hwnd, theme.app_dark);
        if self.is_visible() {
            unsafe {
                let _ = InvalidateRect(self.inner.hwnd, None, false);
            }
        }
    }

    pub fn show_feedback(&mut self, text: impl Into<String>) {
        self.inner.ui.feedback_banner = Some(text.into());
        unsafe {
            let _ = SetTimer(self.inner.hwnd, FEEDBACK_TIMER_ID, 2000, None);
            let _ = InvalidateRect(self.inner.hwnd, None, false);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.inner.hwnd
    }

    fn position_and_resize(&mut self, anchor: Option<RECT>, reposition_from_anchor: bool) {
        position_and_resize_inner(&mut self.inner, anchor, reposition_from_anchor);
    }
}

fn position_and_resize_inner(
    inner: &mut PopoverInner,
    anchor: Option<RECT>,
    reposition_from_anchor: bool,
) {
    let dpi = window_dpi(inner.hwnd);
    let scale = (dpi as f32) / 96.0;
    let width_px = (POPOVER_WIDTH_DIP * scale).round() as i32;
    let height_px = (inner
        .renderer
        .desired_height_dip(&inner.snapshot, &inner.ui)
        * scale)
        .round() as i32;

    if reposition_from_anchor {
        let mut cursor = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut cursor);
        }
        let target_pt = match anchor {
            Some(r) if r.right > r.left && r.bottom > r.top => POINT {
                x: (r.left + r.right) / 2,
                y: (r.top + r.bottom) / 2,
            },
            _ => cursor,
        };

        let monitor = unsafe { MonitorFromPoint(target_pt, MONITOR_DEFAULTTONEAREST) };
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let work = if unsafe { GetMonitorInfoW(monitor, &mut mi).as_bool() } {
            mi.rcWork
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040,
            }
        };

        let margin = (12.0 * scale).round() as i32;
        let mut x = target_pt.x - width_px / 2;
        x = x.clamp(
            work.left + margin,
            (work.right - width_px - margin).max(work.left),
        );

        // Prefer placing above the bottom taskbar; if taskbar is at top, place below it.
        let y = if target_pt.y >= work.bottom - margin {
            (work.bottom - height_px - margin).max(work.top + margin)
        } else if target_pt.y <= work.top + margin {
            work.top + margin
        } else {
            (target_pt.y - height_px - margin).clamp(
                work.top + margin,
                (work.bottom - height_px - margin).max(work.top),
            )
        };

        unsafe {
            let _ = SetWindowPos(
                inner.hwnd,
                HWND_TOPMOST,
                x,
                y,
                width_px,
                height_px,
                SWP_NOACTIVATE,
            );
        }
    } else {
        let mut cur = RECT::default();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(inner.hwnd, &mut cur);
        }
        let anchor_pt = POINT {
            x: (cur.left + cur.right) / 2,
            y: (cur.top + cur.bottom) / 2,
        };
        let monitor = unsafe { MonitorFromPoint(anchor_pt, MONITOR_DEFAULTTONEAREST) };
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let work = if unsafe { GetMonitorInfoW(monitor, &mut mi).as_bool() } {
            mi.rcWork
        } else {
            cur
        };
        let margin = (12.0 * scale).round() as i32;
        let next_compact = inner.ui.is_compact_pinned(&inner.snapshot);
        let header_px = (HEADER_HEIGHT * scale).round() as i32;

        // When toggling compact pinned mode, shift window top by +/- HEADER_HEIGHT so the
        // middle device cards stay frozen at the exact same screen Y coordinate (zero jump/flicker).
        let raw_top = if next_compact && !inner.compact_active {
            cur.top + header_px
        } else if !next_compact && inner.compact_active {
            cur.top - header_px
        } else if inner.ui.pinned {
            cur.top
        } else {
            cur.bottom - height_px
        };
        inner.compact_active = next_compact;

        let new_top = raw_top.clamp(
            work.top + margin,
            (work.bottom - height_px - margin).max(work.top + margin),
        );
        unsafe {
            let _ = SetWindowPos(
                inner.hwnd,
                HWND_TOPMOST,
                cur.left,
                new_top,
                width_px,
                height_px,
                SWP_NOACTIVATE,
            );
        }
    }
}

fn apply_dwm_surface(hwnd: HWND, dark: bool) -> bool {
    unsafe {
        // Extend glass frame across the entire client area so DWM draws backdrop behind D2D.
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

        let dark_flag = BOOL::from(dark);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark_flag as *const BOOL).cast(),
            std::mem::size_of::<BOOL>() as u32,
        );

        let corner: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corner as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );

        let backdrop = DWMSBT_TRANSIENTWINDOW;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&backdrop as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
    }
}

fn window_dpi(hwnd: HWND) -> u32 {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        96
    } else {
        dpi
    }
}

fn paint_inner(inner: &mut PopoverInner) {
    let mut rect = RECT::default();
    unsafe {
        let _ = GetClientRect(inner.hwnd, &mut rect);
    }
    let pw = (rect.right - rect.left).max(1) as u32;
    let ph = (rect.bottom - rect.top).max(1) as u32;
    let dpi = window_dpi(inner.hwnd);
    let _ = inner.renderer.paint(
        PaintSurface {
            hwnd: inner.hwnd,
            pixel_width: pw,
            pixel_height: ph,
            dpi,
            dark: inner.theme.app_dark,
            acrylic_active: inner.acrylic_active,
        },
        &inner.snapshot,
        &mut inner.ui,
    );
}

unsafe extern "system" fn popover_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PopoverInner;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let inner = &mut *ptr;

    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut ps);
            paint_inner(inner);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_ACTIVATE => {
            let state = (wparam.0 & 0xFFFF) as u32;
            if state == WA_INACTIVE && !inner.ui.pinned && IsWindowVisible(hwnd).as_bool() {
                inner.last_auto_hide = Some(Instant::now());
                let _ = ShowWindow(hwnd, SW_HIDE);
                inner.ui.hovered = None;
                inner.ui.feedback_banner = None;
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 as u16 == VK_ESCAPE.0 {
                let _ = KillTimer(hwnd, COLLAPSE_TIMER_ID);
                inner.ui.pinned = false;
                inner.ui.mouse_inside = false;
                inner.compact_active = false;
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x_dip, y_dip) = lparam_to_dip(hwnd, lparam);
            // When pinned, allow smoothly dragging the floating panel by its header bar.
            if inner.ui.pinned
                && !inner.compact_active
                && y_dip <= HEADER_HEIGHT
                && inner.renderer.hit_test(x_dip, y_dip).is_none()
            {
                let _ = KillTimer(hwnd, COLLAPSE_TIMER_ID);
                let _ = ReleaseCapture();
                let _ = SendMessageW(
                    hwnd,
                    WM_NCLBUTTONDOWN,
                    WPARAM(HTCAPTION as usize),
                    LPARAM(0),
                );
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let _ = KillTimer(hwnd, COLLAPSE_TIMER_ID);
            if !inner.tracking_mouse {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut tme);
                inner.tracking_mouse = true;
            }
            if !inner.ui.mouse_inside {
                inner.ui.mouse_inside = true;
                if inner.compact_active {
                    position_and_resize_inner(inner, None, false);
                    paint_inner(inner);
                }
            }
            let (x_dip, y_dip) = lparam_to_dip(hwnd, lparam);
            let hit = inner.renderer.hit_test(x_dip, y_dip);
            if hit.is_some() {
                if let Ok(hand) = LoadCursorW(None, IDC_HAND) {
                    let _ = SetCursor(hand);
                }
            } else if let Ok(arrow) = LoadCursorW(None, IDC_ARROW) {
                let _ = SetCursor(arrow);
            }
            if hit != inner.ui.hovered {
                inner.ui.hovered = hit;
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            inner.tracking_mouse = false;
            if inner.ui.hovered.is_some() {
                inner.ui.hovered = None;
                let _ = InvalidateRect(hwnd, None, false);
            }
            if inner.ui.pinned {
                let _ = SetTimer(hwnd, COLLAPSE_TIMER_ID, 180, None);
            } else {
                inner.ui.mouse_inside = false;
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as u16) as i16 as f32;
            let max_s = inner.renderer.max_scroll();
            if max_s > 0.0 {
                let next = (inner.ui.scroll_offset - (delta / 120.0) * 32.0).clamp(0.0, max_s);
                if (next - inner.ui.scroll_offset).abs() > 0.1 {
                    inner.ui.scroll_offset = next;
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x_dip, y_dip) = lparam_to_dip(hwnd, lparam);
            if let Some(target) = inner.renderer.hit_test(x_dip, y_dip) {
                handle_click(inner, target);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == FEEDBACK_TIMER_ID {
                let _ = KillTimer(hwnd, FEEDBACK_TIMER_ID);
                inner.ui.feedback_banner = None;
                let _ = InvalidateRect(hwnd, None, false);
            } else if wparam.0 == COLLAPSE_TIMER_ID {
                let _ = KillTimer(hwnd, COLLAPSE_TIMER_ID);
                if inner.ui.pinned && !cursor_inside_window(hwnd) {
                    inner.ui.mouse_inside = false;
                    inner.ui.hovered = None;
                    position_and_resize_inner(inner, None, false);
                    paint_inner(inner);
                }
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            position_and_resize_inner(inner, None, false);
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn handle_click(inner: &mut PopoverInner, target: HitTarget) {
    match target {
        HitTarget::TogglePin => {
            inner.ui.pinned = !inner.ui.pinned;
            inner.ui.mouse_inside = true;
            if !inner.ui.pinned {
                unsafe {
                    let _ = KillTimer(inner.hwnd, COLLAPSE_TIMER_ID);
                }
            }
            let feedback = if inner.ui.pinned {
                "已置顶（移出鼠标自动精简）"
            } else {
                "已取消置顶"
            };
            inner.ui.feedback_banner = Some(feedback.to_owned());
            position_and_resize_inner(inner, None, false);
            unsafe {
                let _ = SetWindowPos(
                    inner.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                let _ = SetTimer(inner.hwnd, FEEDBACK_TIMER_ID, 1800, None);
                let _ = InvalidateRect(inner.hwnd, None, false);
            }
        }
        HitTarget::RefreshAll => (inner.on_action)(PopoverAction::RefreshAll),
        HitTarget::OpenSettings | HitTarget::OpenSettingsFromEmpty => {
            inner.ui.view = PopoverView::Settings;
            inner.ui.hovered = None;
            position_and_resize_inner(inner, None, false);
            unsafe {
                let _ = InvalidateRect(inner.hwnd, None, false);
            }
        }
        HitTarget::BackToMain => {
            inner.ui.view = PopoverView::Main;
            inner.ui.hovered = None;
            position_and_resize_inner(inner, None, false);
            unsafe {
                let _ = InvalidateRect(inner.hwnd, None, false);
            }
        }
        HitTarget::ToggleDevice(id) => {
            if inner.ui.expanded_device_id.as_deref() == Some(id.as_str()) {
                inner.ui.expanded_device_id = None;
            } else {
                inner.ui.expanded_device_id = Some(id);
            }
            position_and_resize_inner(inner, None, false);
            unsafe {
                let _ = InvalidateRect(inner.hwnd, None, false);
            }
        }
        HitTarget::RefreshDevice(id) => (inner.on_action)(PopoverAction::RefreshDevice(id)),
        HitTarget::CopyDiagnostic(id) => (inner.on_action)(PopoverAction::CopyDiagnostic(id)),
        HitTarget::SetPollInterval(secs) => (inner.on_action)(PopoverAction::SetPollInterval(secs)),
        HitTarget::ToggleLowBatteryNotify => {
            (inner.on_action)(PopoverAction::ToggleLowBatteryNotify)
        }
        HitTarget::ToggleAutostart => (inner.on_action)(PopoverAction::ToggleAutostart),
        HitTarget::Quit => (inner.on_action)(PopoverAction::Quit),
    }
}

fn cursor_inside_window(hwnd: HWND) -> bool {
    let mut pt = POINT::default();
    let mut rect = RECT::default();
    unsafe {
        if GetCursorPos(&mut pt).is_err()
            || windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect).is_err()
        {
            return false;
        }
    }
    pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.bottom
}

fn lparam_to_dip(hwnd: HWND, lparam: LPARAM) -> (f32, f32) {
    let x_px = (lparam.0 & 0xFFFF) as u16 as i16 as f32;
    let y_px = ((lparam.0 >> 16) & 0xFFFF) as u16 as i16 as f32;
    let scale = (window_dpi(hwnd) as f32) / 96.0;
    (x_px / scale, y_px / scale)
}
