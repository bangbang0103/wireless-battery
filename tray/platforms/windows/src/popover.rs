//! Borderless Windows 11 Fluent 2 Acrylic popover window anchored to the system tray.
//!
//! Features:
//! - True background-free transparent floating mode when pinned and cursor is outside.
//! - Native DWM VSync (`DwmFlush`) animation pacer locked to the display refresh rate
//!   (60/120/144/240Hz) with 0% idle CPU when animations settle.
//! - Zero-jitter layout updates (skips redundant `SetWindowPos` when size is unchanged).

use crate::i18n::Lang;
use crate::icon::load_app_icon;
use crate::renderer::{
    CommunityLinkKind, HitTarget, PaintSurface, PopoverRenderer, PopoverView, UiState,
    UpdateUiStatus, HEADER_HEIGHT, POPOVER_WIDTH_DIP,
};
use crate::theme::SystemTheme;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmFlush, DwmSetWindowAttribute, DWMWA_BORDER_COLOR,
    DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_DONOTROUND, DWMWCP_ROUND, DWM_WINDOW_CORNER_PREFERENCE,
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
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, GetWindowLongPtrW,
    IsWindowVisible, KillTimer, LoadCursorW, PostMessageW, RegisterClassExW, SendMessageW,
    SetCursor, SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWLP_USERDATA, HTCAPTION, HWND_TOPMOST, IDC_ARROW, IDC_HAND, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SW_HIDE, SW_SHOW, WA_INACTIVE, WM_ACTIVATE, WM_APP, WM_DPICHANGED, WM_ERASEBKGND,
    WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCLBUTTONDOWN,
    WM_PAINT, WM_TIMER, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use wireless_battery_tray_core::{LanguageMode, ThemeMode, TraySnapshot};

const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_ANIM_FRAME: u32 = WM_APP + 30;

/// `DWMSBT_NONE` (no system backdrop — transparent glass or solid fallback).
const DWMSBT_NONE: u32 = 1;
/// `DWMSBT_TRANSIENTWINDOW` (Acrylic backdrop designed for Win11 flyouts/popovers).
const DWMSBT_TRANSIENTWINDOW: u32 = 3;
/// `DWMWA_COLOR_NONE`: suppress opaque DWM border so Direct2D renders a true translucent
/// Win11 `SurfaceStrokeColorFlyout` border aligned with `DWMWCP_ROUND` (8 DIPs).
const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;

const FEEDBACK_TIMER_ID: usize = 7001;
const COLLAPSE_TIMER_ID: usize = 7002;

/// User actions emitted from the popover back to the application controller.
#[derive(Debug, Clone)]
pub enum PopoverAction {
    RefreshAll,
    RefreshDevice(String),
    CopyError(String),
    SetThemeMode(ThemeMode),
    SetLanguageMode(LanguageMode),
    SetPollInterval(u64),
    ToggleLowBatteryNotify,
    ToggleLowBatterySilent,
    ToggleAutostart,
    ToggleShowPinButton,
    CheckUpdates,
    OpenUpdateUrl(String),
    OpenCommunityLink(CommunityLinkKind),
    Quit,
}

#[derive(Default)]
struct VsyncControl {
    active: bool,
    shutdown: bool,
}

/// Dedicated on-demand DWM VSync pacer (`DwmFlush`) that wakes the UI thread at the monitor's
/// native refresh rate (60/120/144/240Hz) only while animations are running (0% idle CPU).
struct VsyncPacer {
    control: Arc<(Mutex<VsyncControl>, Condvar)>,
    frame_pending: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl VsyncPacer {
    fn start(hwnd: HWND) -> Self {
        let control = Arc::new((Mutex::new(VsyncControl::default()), Condvar::new()));
        let frame_pending = Arc::new(AtomicBool::new(false));
        let t_control = Arc::clone(&control);
        let t_pending = Arc::clone(&frame_pending);
        let raw_hwnd = hwnd.0;

        let handle = thread::Builder::new()
            .name("wb-tray-vsync".to_owned())
            .spawn(move || loop {
                {
                    let (lock, cvar) = &*t_control;
                    let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                    while !guard.active && !guard.shutdown {
                        guard = cvar.wait(guard).unwrap_or_else(|e| e.into_inner());
                    }
                    if guard.shutdown {
                        break;
                    }
                }

                // Wait for the exact next DWM compositor VBlank (supports 60/120/144/240Hz).
                let flushed = unsafe { DwmFlush().is_ok() };
                if !flushed {
                    thread::sleep(Duration::from_millis(8));
                }

                let still_active = {
                    let (lock, _) = &*t_control;
                    let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                    guard.active && !guard.shutdown
                };
                if still_active && !t_pending.swap(true, Ordering::AcqRel) {
                    unsafe {
                        let _ = PostMessageW(HWND(raw_hwnd), WM_ANIM_FRAME, WPARAM(0), LPARAM(0));
                    }
                }
            })
            .ok();

        Self {
            control,
            frame_pending,
            thread: handle,
        }
    }

    fn set_active(&self, active: bool) {
        let (lock, cvar) = &*self.control;
        let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        if guard.active != active {
            guard.active = active;
            if active {
                self.frame_pending.store(false, Ordering::Release);
                cvar.notify_one();
            } else {
                self.frame_pending.store(false, Ordering::Release);
            }
        }
    }

    fn mark_frame_consumed(&self) {
        self.frame_pending.store(false, Ordering::Release);
    }
}

impl Drop for VsyncPacer {
    fn drop(&mut self) {
        {
            let (lock, cvar) = &*self.control;
            let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            guard.shutdown = true;
            guard.active = false;
            cvar.notify_one();
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

type ActionCallback = Box<dyn Fn(PopoverAction)>;

struct PopoverInner {
    hwnd: HWND,
    renderer: PopoverRenderer,
    ui: UiState,
    snapshot: TraySnapshot,
    theme: SystemTheme,
    effective_dark: bool,
    acrylic_active: bool,
    compact_active: bool,
    dwm_compact_applied: bool,
    last_size_px: (i32, i32),
    tracking_mouse: bool,
    vsync: VsyncPacer,
    last_auto_hide: Option<Instant>,
}

struct PopoverCell {
    hwnd: HWND,
    core: RefCell<PopoverInner>,
    on_action: ActionCallback,
}

/// Owning handle for the native Direct2D + DWM Acrylic popover window.
pub struct PopoverWindow {
    cell: Box<PopoverCell>,
}

impl Drop for PopoverWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = KillTimer(self.cell.hwnd, COLLAPSE_TIMER_ID);
            let _ = KillTimer(self.cell.hwnd, FEEDBACK_TIMER_ID);
            let _ = SetWindowLongPtrW(self.cell.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.cell.hwnd);
        }
    }
}

impl PopoverWindow {
    pub fn create(
        initial_snapshot: TraySnapshot,
        theme: SystemTheme,
        on_action: impl Fn(PopoverAction) + 'static,
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
                POPOVER_WIDTH_DIP as i32,
                280,
                None,
                None,
                hinstance,
                None,
            );
            if hwnd.0 == 0 {
                return Err(windows::core::Error::from_win32());
            }

            let effective_dark = theme.resolve_dark(initial_snapshot.config.theme_mode);
            let acrylic_active =
                apply_dwm_surface(hwnd, effective_dark, theme.transparency_enabled, false);

            let mut ui = UiState::default();
            ui.sync_from_snapshot(&initial_snapshot, true);
            let vsync = VsyncPacer::start(hwnd);

            let cell = Box::new(PopoverCell {
                hwnd,
                core: RefCell::new(PopoverInner {
                    hwnd,
                    renderer,
                    ui,
                    snapshot: initial_snapshot,
                    theme,
                    effective_dark,
                    acrylic_active,
                    compact_active: false,
                    dwm_compact_applied: false,
                    last_size_px: (POPOVER_WIDTH_DIP as i32, 280),
                    tracking_mouse: false,
                    vsync,
                    last_auto_hide: None,
                }),
                on_action: Box::new(on_action),
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*cell as *const PopoverCell) as isize);

            // Pre-warm Direct2D render target and DirectWrite font caches while hidden so the
            // very first click renders immediately with zero white flash.
            if let Ok(mut inner) = cell.core.try_borrow_mut() {
                paint_inner(&mut inner);
            }

            Ok(Self { cell })
        }
    }

    pub fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.cell.hwnd).as_bool() }
    }

    /// Toggle visibility cleanly, debouncing click-on-tray-icon right after focus-loss hide.
    pub fn toggle(&self, anchor: Option<RECT>) {
        if self.is_visible() {
            self.hide();
            return;
        }
        let skip = {
            let Ok(mut inner) = self.cell.core.try_borrow_mut() else {
                return;
            };
            if inner
                .last_auto_hide
                .is_some_and(|last| last.elapsed().as_millis() < 220)
            {
                inner.last_auto_hide = None;
                true
            } else {
                false
            }
        };
        if !skip {
            self.show(anchor);
        }
    }

    pub fn show(&self, anchor: Option<RECT>) {
        let pos = {
            let Ok(mut inner) = self.cell.core.try_borrow_mut() else {
                return;
            };
            inner.ui.view = PopoverView::Main;
            inner.ui.close_dropdown_menu();
            inner.ui.hovered = None;
            inner.ui.pressed = None;
            inner.ui.mouse_inside = false;
            inner.compact_active = false;
            ensure_dwm_surface_mode(&mut inner, false);
            let snap = inner.snapshot.clone();
            inner.ui.sync_from_snapshot(&snap, true);
            // Position and render synchronously while still hidden to eliminate any white flash.
            compute_position_and_resize(&mut inner, anchor, true)
        };
        apply_window_pos(self.cell.hwnd, pos);
        if let Ok(mut inner) = self.cell.core.try_borrow_mut() {
            paint_inner(&mut inner);
        }
        unsafe {
            let _ = ShowWindow(self.cell.hwnd, SW_SHOW);
            let _ = SetForegroundWindow(self.cell.hwnd);
            let _ = UpdateWindow(self.cell.hwnd);
        }
        if let Ok(mut inner) = self.cell.core.try_borrow_mut() {
            sync_vsync_pacer(&mut inner);
        }
    }

    pub fn hide(&self) {
        if let Ok(inner) = self.cell.core.try_borrow() {
            inner.vsync.set_active(false);
        }
        unsafe {
            let _ = KillTimer(self.cell.hwnd, COLLAPSE_TIMER_ID);
            let _ = ShowWindow(self.cell.hwnd, SW_HIDE);
        }
        if let Ok(mut inner) = self.cell.core.try_borrow_mut() {
            inner.ui.pinned = false;
            inner.ui.mouse_inside = false;
            inner.ui.close_dropdown_menu();
            inner.compact_active = false;
            ensure_dwm_surface_mode(&mut inner, false);
            inner.ui.hovered = None;
            inner.ui.pressed = None;
            inner.ui.feedback_banner = None;
        }
    }

    pub fn set_snapshot(&self, snapshot: TraySnapshot) {
        let visible = self.is_visible();
        let (pos, needs_paint) = {
            let Ok(mut inner) = self.cell.core.try_borrow_mut() else {
                return;
            };
            let next_dark = inner.theme.resolve_dark(snapshot.config.theme_mode);
            let dark_changed = next_dark != inner.effective_dark;
            inner.snapshot = snapshot;
            let snap = inner.snapshot.clone();
            inner.ui.sync_from_snapshot(&snap, !visible);
            if dark_changed {
                inner.effective_dark = next_dark;
                inner.acrylic_active = apply_dwm_surface(
                    inner.hwnd,
                    next_dark,
                    inner.theme.transparency_enabled,
                    inner.dwm_compact_applied,
                );
            }
            let pos = if visible {
                compute_position_and_resize(&mut inner, None, false)
            } else {
                None
            };
            (pos, visible || dark_changed)
        };
        apply_window_pos(self.cell.hwnd, pos);
        if needs_paint {
            if let Ok(mut inner) = self.cell.core.try_borrow_mut() {
                if visible {
                    sync_vsync_pacer(&mut inner);
                }
                paint_inner(&mut inner);
            }
        }
    }

    pub fn set_theme(&self, theme: SystemTheme) {
        let Ok(mut inner) = self.cell.core.try_borrow_mut() else {
            return;
        };
        if inner.theme == theme {
            return;
        }
        let prev_transparency = inner.theme.transparency_enabled;
        inner.theme = theme;
        let next_dark = theme.resolve_dark(inner.snapshot.config.theme_mode);
        if next_dark != inner.effective_dark || prev_transparency != theme.transparency_enabled {
            inner.effective_dark = next_dark;
            inner.acrylic_active = apply_dwm_surface(
                inner.hwnd,
                next_dark,
                theme.transparency_enabled,
                inner.dwm_compact_applied,
            );
        }
        paint_inner(&mut inner);
    }

    pub fn show_feedback(&self, text: impl Into<String>) {
        if let Ok(mut inner) = self.cell.core.try_borrow_mut() {
            inner.ui.feedback_banner = Some(text.into());
        }
        unsafe {
            let _ = SetTimer(self.cell.hwnd, FEEDBACK_TIMER_ID, 2000, None);
            let _ = InvalidateRect(self.cell.hwnd, None, false);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.cell.hwnd
    }

    pub fn set_update_status(&self, status: UpdateUiStatus) {
        let visible = self.is_visible();
        if let Ok(mut inner) = self.cell.core.try_borrow_mut() {
            inner.ui.update_status = status;
            if visible {
                paint_inner(&mut inner);
            }
        }
    }
}

struct WindowPosUpdate {
    x: i32,
    y: i32,
    width_px: i32,
    height_px: i32,
    flags: u32,
}

fn apply_window_pos(hwnd: HWND, update: Option<WindowPosUpdate>) {
    let Some(upd) = update else {
        return;
    };
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            upd.x,
            upd.y,
            upd.width_px,
            upd.height_px,
            windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(upd.flags),
        );
    }
}

fn sync_vsync_pacer(inner: &mut PopoverInner) {
    let visible = unsafe { IsWindowVisible(inner.hwnd).as_bool() };
    let needs = visible && inner.ui.needs_animation_frame(inner.snapshot.refreshing);
    inner.vsync.set_active(needs);
}

fn ensure_dwm_surface_mode(inner: &mut PopoverInner, compact_no_bg: bool) {
    if inner.dwm_compact_applied != compact_no_bg {
        inner.dwm_compact_applied = compact_no_bg;
        inner.acrylic_active = apply_dwm_surface(
            inner.hwnd,
            inner.effective_dark,
            inner.theme.transparency_enabled,
            compact_no_bg,
        );
    }
}

fn compute_position_and_resize(
    inner: &mut PopoverInner,
    anchor: Option<RECT>,
    reposition_from_anchor: bool,
) -> Option<WindowPosUpdate> {
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

        inner.last_size_px = (width_px, height_px);
        Some(WindowPosUpdate {
            x,
            y,
            width_px,
            height_px,
            flags: SWP_NOACTIVATE.0,
        })
    } else {
        let next_compact = inner.ui.is_compact_pinned(&inner.snapshot);
        ensure_dwm_surface_mode(inner, next_compact);

        // Skip redundant SetWindowPos when size & compact state haven't changed (prevents DWM jitter).
        if (width_px, height_px) == inner.last_size_px && next_compact == inner.compact_active {
            return None;
        }

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
        let header_px = (HEADER_HEIGHT * scale).round() as i32;

        // When toggling compact pinned mode, shift window top by +/- HEADER_HEIGHT so the
        // device rows stay frozen at the exact same screen Y coordinate (zero jump/flicker).
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
        inner.last_size_px = (width_px, height_px);

        let new_top = raw_top.clamp(
            work.top + margin,
            (work.bottom - height_px - margin).max(work.top + margin),
        );
        Some(WindowPosUpdate {
            x: cur.left,
            y: new_top,
            width_px,
            height_px,
            flags: SWP_NOACTIVATE.0,
        })
    }
}

fn apply_dwm_surface(
    hwnd: HWND,
    dark: bool,
    transparency_enabled: bool,
    compact_no_bg: bool,
) -> bool {
    unsafe {
        // Extend glass frame across the entire client area so DWM composites our premultiplied
        // alpha channel cleanly (needed for both Acrylic backdrop and 100% transparent compact mode).
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

        let corner: DWM_WINDOW_CORNER_PREFERENCE = if compact_no_bg {
            // Suppress DWM rounded frame & shadow box in background-free compact pinned mode.
            DWMWCP_DONOTROUND
        } else {
            DWMWCP_ROUND
        };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corner as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );

        let no_border = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            (&no_border as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        );

        let backdrop = if compact_no_bg || !transparency_enabled {
            DWMSBT_NONE
        } else {
            DWMSBT_TRANSIENTWINDOW
        };
        let backdrop_ok = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&backdrop as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok();

        !compact_no_bg && transparency_enabled && backdrop_ok
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
            system_theme: inner.theme,
            effective_dark: inner.effective_dark,
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
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const PopoverCell;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let cell = &*ptr;

    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut ps);
            if let Ok(mut inner) = cell.core.try_borrow_mut() {
                paint_inner(&mut inner);
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_ANIM_FRAME => {
            let visible = IsWindowVisible(hwnd).as_bool();
            if let Ok(mut inner) = cell.core.try_borrow_mut() {
                inner.vsync.mark_frame_consumed();
                if visible {
                    let mut shrunk = false;
                    if inner.ui.pending_settings_shrink
                        && !inner.ui.settings_sections.is_animating(Instant::now())
                    {
                        inner.ui.pending_settings_shrink = false;
                        let upd = compute_position_and_resize(&mut inner, None, false);
                        apply_window_pos(hwnd, upd);
                        shrunk = true;
                    }
                    paint_inner(&mut inner);
                    if !shrunk
                        && inner.ui.pending_settings_shrink
                        && !inner.ui.settings_sections.is_animating(Instant::now())
                    {
                        inner.ui.pending_settings_shrink = false;
                        let upd = compute_position_and_resize(&mut inner, None, false);
                        apply_window_pos(hwnd, upd);
                        paint_inner(&mut inner);
                    }
                }
                sync_vsync_pacer(&mut inner);
            }
            LRESULT(0)
        }
        WM_ACTIVATE => {
            let state = (wparam.0 & 0xFFFF) as u32;
            let should_hide = if state == WA_INACTIVE && IsWindowVisible(hwnd).as_bool() {
                let Ok(mut inner) = cell.core.try_borrow_mut() else {
                    return LRESULT(0);
                };
                if inner.ui.pinned {
                    false
                } else {
                    inner.vsync.set_active(false);
                    inner.last_auto_hide = Some(Instant::now());
                    inner.ui.close_dropdown_menu();
                    inner.ui.hovered = None;
                    inner.ui.pressed = None;
                    inner.ui.feedback_banner = None;
                    true
                }
            } else {
                false
            };
            if should_hide {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 as u16 == VK_ESCAPE.0 {
                let should_hide = {
                    let Ok(mut inner) = cell.core.try_borrow_mut() else {
                        return LRESULT(0);
                    };
                    if inner.ui.open_dropdown.is_some() {
                        inner.ui.close_dropdown_menu();
                        sync_vsync_pacer(&mut inner);
                        paint_inner(&mut inner);
                        false
                    } else {
                        inner.vsync.set_active(false);
                        let _ = KillTimer(hwnd, COLLAPSE_TIMER_ID);
                        inner.ui.pinned = false;
                        inner.ui.mouse_inside = false;
                        inner.ui.pressed = None;
                        inner.compact_active = false;
                        ensure_dwm_surface_mode(&mut inner, false);
                        true
                    }
                };
                if should_hide {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x_dip, y_dip) = lparam_to_dip(hwnd, lparam);
            let should_drag = {
                let Ok(mut inner) = cell.core.try_borrow_mut() else {
                    return LRESULT(0);
                };
                let hit = inner.renderer.hit_test(x_dip, y_dip);
                if hit.is_some() {
                    inner.ui.pressed = hit;
                    paint_inner(&mut inner);
                    false
                } else {
                    inner.ui.pinned && !inner.compact_active && y_dip <= HEADER_HEIGHT
                }
            };
            if should_drag {
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
            let (x_dip, y_dip) = lparam_to_dip(hwnd, lparam);
            let (pos_upd, interactive_cursor, needs_paint) = {
                let Ok(mut inner) = cell.core.try_borrow_mut() else {
                    return LRESULT(0);
                };
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
                let mut pos_upd = None;
                let mut needs_paint = false;
                if !inner.ui.mouse_inside {
                    inner.ui.mouse_inside = true;
                    if inner.compact_active {
                        pos_upd = compute_position_and_resize(&mut inner, None, false);
                        needs_paint = true;
                    }
                }
                let hit = inner.renderer.hit_test(x_dip, y_dip);
                let interactive = hit.is_some() && !matches!(hit, Some(HitTarget::CloseDropdown));
                if hit != inner.ui.hovered {
                    inner.ui.hovered = hit;
                    needs_paint = true;
                }
                (pos_upd, interactive, needs_paint)
            };
            if interactive_cursor {
                if let Ok(hand) = LoadCursorW(None, IDC_HAND) {
                    let _ = SetCursor(hand);
                }
            } else if let Ok(arrow) = LoadCursorW(None, IDC_ARROW) {
                let _ = SetCursor(arrow);
            }
            apply_window_pos(hwnd, pos_upd);
            if needs_paint {
                if let Ok(mut inner) = cell.core.try_borrow_mut() {
                    paint_inner(&mut inner);
                }
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if let Ok(mut inner) = cell.core.try_borrow_mut() {
                inner.tracking_mouse = false;
                if inner.ui.hovered.is_some() || inner.ui.pressed.is_some() {
                    inner.ui.hovered = None;
                    inner.ui.pressed = None;
                    paint_inner(&mut inner);
                }
                if inner.ui.pinned {
                    let _ = SetTimer(hwnd, COLLAPSE_TIMER_ID, 180, None);
                } else {
                    inner.ui.mouse_inside = false;
                }
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            if let Ok(mut inner) = cell.core.try_borrow_mut() {
                if inner.ui.open_dropdown.is_none() {
                    let delta = ((wparam.0 >> 16) as u16) as i16 as f32;
                    let max_s = inner.renderer.max_scroll();
                    if max_s > 0.0 {
                        let next =
                            (inner.ui.scroll_offset - (delta / 120.0) * 32.0).clamp(0.0, max_s);
                        if (next - inner.ui.scroll_offset).abs() > 0.1 {
                            inner.ui.scroll_offset = next;
                            paint_inner(&mut inner);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x_dip, y_dip) = lparam_to_dip(hwnd, lparam);
            let outcome = {
                let Ok(mut inner) = cell.core.try_borrow_mut() else {
                    return LRESULT(0);
                };
                let pressed = inner.ui.pressed.take();
                if let Some(target) = inner.renderer.hit_test(x_dip, y_dip) {
                    if pressed.is_none() || pressed.as_ref() == Some(&target) {
                        handle_click(&mut inner, target)
                    } else if pressed.is_some() {
                        ClickOutcome {
                            repaint: true,
                            ..Default::default()
                        }
                    } else {
                        ClickOutcome::default()
                    }
                } else if pressed.is_some() {
                    ClickOutcome {
                        repaint: true,
                        ..Default::default()
                    }
                } else {
                    ClickOutcome::default()
                }
            };
            apply_window_pos(hwnd, outcome.pos);
            if outcome.set_topmost_timer {
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                let _ = SetTimer(hwnd, FEEDBACK_TIMER_ID, 1800, None);
            }
            if outcome.repaint {
                if let Ok(mut inner) = cell.core.try_borrow_mut() {
                    paint_inner(&mut inner);
                }
            }
            if let Some(action) = outcome.action {
                (cell.on_action)(action);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == FEEDBACK_TIMER_ID {
                let _ = KillTimer(hwnd, FEEDBACK_TIMER_ID);
                if let Ok(mut inner) = cell.core.try_borrow_mut() {
                    inner.ui.feedback_banner = None;
                    paint_inner(&mut inner);
                }
            } else if wparam.0 == COLLAPSE_TIMER_ID {
                let _ = KillTimer(hwnd, COLLAPSE_TIMER_ID);
                let inside = cursor_inside_window(hwnd);
                let pos_upd = if !inside {
                    let Ok(mut inner) = cell.core.try_borrow_mut() else {
                        return LRESULT(0);
                    };
                    if inner.ui.pinned {
                        inner.ui.mouse_inside = false;
                        inner.ui.hovered = None;
                        inner.ui.pressed = None;
                        compute_position_and_resize(&mut inner, None, false)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if !inside {
                    apply_window_pos(hwnd, pos_upd);
                    if let Ok(mut inner) = cell.core.try_borrow_mut() {
                        if inner.ui.pinned {
                            paint_inner(&mut inner);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let pos_upd = {
                let Ok(mut inner) = cell.core.try_borrow_mut() else {
                    return LRESULT(0);
                };
                compute_position_and_resize(&mut inner, None, false)
            };
            apply_window_pos(hwnd, pos_upd);
            if let Ok(mut inner) = cell.core.try_borrow_mut() {
                paint_inner(&mut inner);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[derive(Default)]
struct ClickOutcome {
    action: Option<PopoverAction>,
    pos: Option<WindowPosUpdate>,
    set_topmost_timer: bool,
    repaint: bool,
}

fn handle_click(inner: &mut PopoverInner, target: HitTarget) -> ClickOutcome {
    let mut out = ClickOutcome::default();
    match target {
        HitTarget::TogglePin => {
            inner.ui.pinned = !inner.ui.pinned;
            inner.ui.mouse_inside = true;
            if !inner.ui.pinned {
                unsafe {
                    let _ = KillTimer(inner.hwnd, COLLAPSE_TIMER_ID);
                }
            }
            let lang = Lang::resolve(inner.snapshot.config.language);
            let feedback = if inner.ui.pinned {
                lang.toast_pinned()
            } else {
                lang.toast_unpinned()
            };
            inner.ui.feedback_banner = Some(feedback.to_owned());
            out.pos = compute_position_and_resize(inner, None, false);
            out.set_topmost_timer = true;
            out.repaint = true;
        }
        HitTarget::RefreshAll => out.action = Some(PopoverAction::RefreshAll),
        HitTarget::OpenSettings => {
            inner.ui.view = PopoverView::Settings;
            inner.ui.scroll_offset = 0.0;
            inner.ui.close_dropdown_menu();
            inner.ui.hovered = None;
            inner.ui.pressed = None;
            let snap = inner.snapshot.clone();
            inner.ui.sync_from_snapshot(&snap, true);
            out.pos = compute_position_and_resize(inner, None, false);
            out.repaint = true;
        }
        HitTarget::BackToMain => {
            inner.ui.view = PopoverView::Main;
            inner.ui.scroll_offset = 0.0;
            inner.ui.pending_settings_shrink = false;
            inner.ui.close_dropdown_menu();
            inner.ui.hovered = None;
            inner.ui.pressed = None;
            out.pos = compute_position_and_resize(inner, None, false);
            out.repaint = true;
        }
        HitTarget::ToggleDropdown(which) => {
            if inner.ui.open_dropdown == Some(which) {
                inner.ui.close_dropdown_menu();
            } else {
                inner.ui.open_dropdown_menu(which);
            }
            sync_vsync_pacer(inner);
            out.repaint = true;
        }
        HitTarget::CloseDropdown => {
            inner.ui.close_dropdown_menu();
            sync_vsync_pacer(inner);
            out.repaint = true;
        }
        HitTarget::SetThemeMode(mode) => {
            inner.ui.close_dropdown_menu();
            sync_vsync_pacer(inner);
            out.action = Some(PopoverAction::SetThemeMode(mode));
        }
        HitTarget::SetLanguageMode(mode) => {
            inner.ui.close_dropdown_menu();
            sync_vsync_pacer(inner);
            out.action = Some(PopoverAction::SetLanguageMode(mode));
        }
        HitTarget::SetPollInterval(secs) => {
            inner.ui.close_dropdown_menu();
            sync_vsync_pacer(inner);
            out.action = Some(PopoverAction::SetPollInterval(secs));
        }
        HitTarget::ToggleDevice(id) => {
            if inner.ui.expanded_device_id.as_deref() == Some(id.as_str()) {
                inner.ui.expanded_device_id = None;
            } else {
                inner.ui.expanded_device_id = Some(id);
                inner.ui.trigger_drawer_reveal();
            }
            out.pos = compute_position_and_resize(inner, None, false);
            sync_vsync_pacer(inner);
            out.repaint = true;
        }
        HitTarget::RefreshDevice(id) => out.action = Some(PopoverAction::RefreshDevice(id)),
        HitTarget::CopyError(id) => out.action = Some(PopoverAction::CopyError(id)),
        HitTarget::ToggleLowBatteryNotify => {
            out.action = Some(PopoverAction::ToggleLowBatteryNotify);
        }
        HitTarget::ToggleLowBatterySilent => {
            out.action = Some(PopoverAction::ToggleLowBatterySilent);
        }
        HitTarget::ToggleAutostart => out.action = Some(PopoverAction::ToggleAutostart),
        HitTarget::ToggleShowPinButton => {
            if inner.snapshot.config.show_pin_button && inner.ui.pinned {
                inner.ui.pinned = false;
                inner.compact_active = false;
                unsafe {
                    let _ = KillTimer(inner.hwnd, COLLAPSE_TIMER_ID);
                }
            }
            out.action = Some(PopoverAction::ToggleShowPinButton);
        }
        HitTarget::ToggleSettingsSection(section) => {
            let now = Instant::now();
            let was_expanded = inner.ui.settings_sections.is_expanded(section);
            inner.ui.close_dropdown_menu();
            inner.ui.settings_sections.toggle(section, now);
            if was_expanded {
                // Defer window shrink until the collapse animation finishes so Acrylic never flickers.
                inner.ui.pending_settings_shrink = true;
            } else {
                inner.ui.pending_settings_shrink = false;
                out.pos = compute_position_and_resize(inner, None, false);
            }
            sync_vsync_pacer(inner);
            out.repaint = true;
        }
        HitTarget::CheckUpdates => match &inner.ui.update_status {
            UpdateUiStatus::Checking => {}
            UpdateUiStatus::Available { url, .. } => {
                out.action = Some(PopoverAction::OpenUpdateUrl(url.clone()));
            }
            UpdateUiStatus::Idle | UpdateUiStatus::UpToDate | UpdateUiStatus::Failed => {
                inner.ui.update_status = UpdateUiStatus::Checking;
                out.repaint = true;
                out.action = Some(PopoverAction::CheckUpdates);
            }
        },
        HitTarget::OpenCommunityLink(kind) => {
            out.action = Some(PopoverAction::OpenCommunityLink(kind));
        }
        HitTarget::Quit => out.action = Some(PopoverAction::Quit),
    }
    out
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
