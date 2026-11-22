//! Ephemeral UI state, hit-test target identifiers, and cubic ease-out micro-animations.

use std::time::Instant;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use wireless_battery_tray_core::{LanguageMode, ThemeMode, TraySnapshot};

/// Logical popover width in DIPs (matches Windows 11 Quick Settings / Bluetooth flyout width).
pub const POPOVER_WIDTH_DIP: f32 = 360.0;
pub const POPOVER_MAX_HEIGHT_DIP: f32 = 560.0;
pub const HEADER_HEIGHT: f32 = 52.0;

pub(super) const FOOTER_HEIGHT: f32 = 48.0;
pub(super) const OUTER_PADDING: f32 = 16.0;
pub(super) const LIST_VERTICAL_PAD: f32 = 6.0;
pub(super) const CARD_RADIUS: f32 = 8.0;
pub(super) const CONTROL_RADIUS: f32 = 4.0;
pub(super) const WINDOW_CORNER_RADIUS: f32 = 8.0;
pub(super) const ROW_COLLAPSED_H: f32 = 66.0;
pub(super) const ROW_EXPANDED_EXTRA_H: f32 = 106.0;
pub(super) const SETTINGS_CARD_H: f32 = 56.0;
pub(super) const SETTINGS_SUBROW_H: f32 = 48.0;
pub(super) const SETTINGS_CARD_GAP: f32 = 4.0;

/// Active view inside the single-popover experience.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverView {
    Main,
    Settings,
}

/// Currently open WinUI 3 `ComboBox` dropdown in the Settings view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenDropdown {
    ThemeMode,
    Language,
    PollInterval,
}

/// Interactive targets registered during every paint pass for pixel-accurate hit-testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitTarget {
    TogglePin,
    RefreshAll,
    OpenSettings,
    BackToMain,
    ToggleDropdown(OpenDropdown),
    CloseDropdown,
    SetThemeMode(ThemeMode),
    SetLanguageMode(LanguageMode),
    Quit,
    ToggleDevice(String),
    RefreshDevice(String),
    CopyError(String),
    SetPollInterval(u64),
    ToggleLowBatteryNotify,
    ToggleLowBatterySilent,
    ToggleAutostart,
    CheckUpdates,
}

/// UI state of the manual update check button inside the Settings view.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateUiStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available {
        latest_version: String,
        url: String,
    },
    Failed,
}

#[derive(Debug, Clone)]
pub(super) struct HitRegion {
    pub(super) rect: D2D_RECT_F,
    pub(super) target: HitTarget,
}

/// Smooth cubic ease-out scalar interpolator for zero-overhead WinUI 3 micro-animations.
#[derive(Debug, Clone, Copy)]
pub struct ScalarAnim {
    from: f32,
    to: f32,
    start: Instant,
    duration_ms: u32,
    initialized: bool,
}

impl ScalarAnim {
    pub(super) fn settled(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            start: Instant::now(),
            duration_ms: 1,
            initialized: false,
        }
    }

    pub(super) fn snap(&mut self, value: f32) {
        self.from = value;
        self.to = value;
        self.initialized = true;
    }

    pub(super) fn animate_to(&mut self, target: f32, duration_ms: u32, now: Instant) {
        if !self.initialized {
            self.snap(target);
            return;
        }
        if (self.to - target).abs() < 1e-4 {
            return;
        }
        self.from = self.value(now);
        self.to = target;
        self.start = now;
        self.duration_ms = duration_ms.max(1);
    }

    pub(super) fn restart(&mut self, from: f32, to: f32, duration_ms: u32, now: Instant) {
        self.from = from;
        self.to = to;
        self.start = now;
        self.duration_ms = duration_ms.max(1);
        self.initialized = true;
    }

    pub(super) fn value(&self, now: Instant) -> f32 {
        if (self.to - self.from).abs() < 1e-4 {
            return self.to;
        }
        let elapsed_ms = now.saturating_duration_since(self.start).as_secs_f32() * 1000.0;
        let t = (elapsed_ms / (self.duration_ms as f32)).clamp(0.0, 1.0);
        // WinUI 3 Fluent fast-out slow-in cubic deceleration curve
        let eased = 1.0 - (1.0 - t).powi(3);
        self.from + (self.to - self.from) * eased
    }

    pub(super) fn is_active(&self, now: Instant) -> bool {
        if (self.to - self.from).abs() < 1e-4 {
            return false;
        }
        let elapsed_ms = now.saturating_duration_since(self.start).as_secs_f32() * 1000.0;
        elapsed_ms < (self.duration_ms as f32)
    }
}

/// Ephemeral UI and micro-animation state owned by the popover window.
pub struct UiState {
    pub view: PopoverView,
    pub open_dropdown: Option<OpenDropdown>,
    pub pinned: bool,
    pub mouse_inside: bool,
    pub expanded_device_id: Option<String>,
    pub scroll_offset: f32,
    pub hovered: Option<HitTarget>,
    pub pressed: Option<HitTarget>,
    pub feedback_banner: Option<String>,
    pub update_status: UpdateUiStatus,
    pub(super) toggle_notify_anim: ScalarAnim,
    pub(super) toggle_silent_anim: ScalarAnim,
    pub(super) toggle_autostart_anim: ScalarAnim,
    pub(super) dropdown_anim: ScalarAnim,
    pub(super) drawer_reveal_anim: ScalarAnim,
    pub(super) refresh_anim_start: Option<Instant>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            view: PopoverView::Main,
            open_dropdown: None,
            pinned: false,
            mouse_inside: false,
            expanded_device_id: None,
            scroll_offset: 0.0,
            hovered: None,
            pressed: None,
            feedback_banner: None,
            update_status: UpdateUiStatus::Idle,
            toggle_notify_anim: ScalarAnim::settled(1.0),
            toggle_silent_anim: ScalarAnim::settled(0.0),
            toggle_autostart_anim: ScalarAnim::settled(0.0),
            dropdown_anim: ScalarAnim::settled(0.0),
            drawer_reveal_anim: ScalarAnim::settled(1.0),
            refresh_anim_start: None,
        }
    }
}

impl UiState {
    /// True when pinned in compact floating mode (cursor outside): hides window background,
    /// header, and footer, floating only the clean divider-separated device list.
    pub fn is_compact_pinned(&self, snapshot: &TraySnapshot) -> bool {
        self.pinned
            && !self.mouse_inside
            && self.view == PopoverView::Main
            && !snapshot.devices.is_empty()
    }

    /// Synchronize animation targets to the latest snapshot.
    /// Pass `instant = true` when opening the window so controls appear settled immediately.
    pub fn sync_from_snapshot(&mut self, snapshot: &TraySnapshot, instant: bool) {
        let now = Instant::now();
        let notify_t = if snapshot.config.low_battery_notify {
            1.0
        } else {
            0.0
        };
        let silent_t = if snapshot.config.low_battery_silent {
            1.0
        } else {
            0.0
        };
        let auto_t = if snapshot.config.autostart { 1.0 } else { 0.0 };

        if instant {
            self.toggle_notify_anim.snap(notify_t);
            self.toggle_silent_anim.snap(silent_t);
            self.toggle_autostart_anim.snap(auto_t);
            self.drawer_reveal_anim.snap(1.0);
        } else {
            self.toggle_notify_anim.animate_to(notify_t, 160, now);
            self.toggle_silent_anim.animate_to(silent_t, 160, now);
            self.toggle_autostart_anim.animate_to(auto_t, 160, now);
        }

        if snapshot.refreshing {
            if self.refresh_anim_start.is_none() {
                self.refresh_anim_start = Some(now);
            }
        } else {
            self.refresh_anim_start = None;
        }
    }

    /// Open a WinUI 3 `ComboBox` dropdown with a smooth slide-down reveal animation.
    pub fn open_dropdown_menu(&mut self, which: OpenDropdown) {
        self.open_dropdown = Some(which);
        self.dropdown_anim.restart(0.0, 1.0, 140, Instant::now());
    }

    /// Close any open `ComboBox` dropdown.
    pub fn close_dropdown_menu(&mut self) {
        self.open_dropdown = None;
        self.dropdown_anim.snap(0.0);
    }

    /// Trigger a smooth reveal fade for an expanded device drawer.
    pub fn trigger_drawer_reveal(&mut self) {
        self.drawer_reveal_anim
            .restart(0.0, 1.0, 150, Instant::now());
    }

    /// Returns true while any micro-animation or refreshing indicator requires VSync frames.
    pub fn needs_animation_frame(&self, refreshing: bool) -> bool {
        if refreshing {
            return true;
        }
        let now = Instant::now();
        self.toggle_notify_anim.is_active(now)
            || self.toggle_silent_anim.is_active(now)
            || self.toggle_autostart_anim.is_active(now)
            || self.dropdown_anim.is_active(now)
            || self.drawer_reveal_anim.is_active(now)
    }
}
