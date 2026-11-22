//! Direct2D + DirectWrite renderer and hit-testing engine for the Windows 11 Fluent 2 popover.
//!
//! Aligned to official WinUI 3 / Windows 11 Quick Settings & Settings metrics:
//! - Main flyout: seamless divider-separated device rows (`1px` hairline between devices),
//!   minimalist `52px` header & `48px` footer; compact pinned mode becomes 100% background-free.
//! - Settings view: flat WinUI 3 `SettingsCard` rows (`4px` gap) using native-style
//!   `ComboBox` dropdown flyouts (Theme, Language, Poll Interval incl. 1 hour), a composite
//!   Low Battery Alert card with indented Silent Notifications sub-option, and Launch at Startup.
//! - Bilingual Simplified Chinese & English UI (`i18n::Lang`).
//! - Silky DWM-VSync-paced micro-animations (60/120/144/240Hz lock-step, 0% idle CPU).

use crate::i18n::Lang;
use crate::icon::resolve_icon_font_family;
use crate::theme::{lerp_color, with_alpha, Palette, SystemTheme};
use std::time::Instant;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F,
    D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES,
    D2D1_PRESENT_OPTIONS_IMMEDIATELY, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_ROUNDED_RECT, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_MEDIUM,
    DWRITE_FONT_WEIGHT_REGULAR, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWRITE_WORD_WRAPPING_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use wireless_battery::{BatteryLevel, ChargeState, Freshness, Precision, Reachability};
use wireless_battery_tray_core::{
    DeviceKind, DeviceView, LanguageMode, ThemeMode, TrayIconState, TraySnapshot,
};

/// Logical popover width in DIPs (matches Windows 11 Quick Settings / Bluetooth flyout width).
pub const POPOVER_WIDTH_DIP: f32 = 360.0;
pub const POPOVER_MAX_HEIGHT_DIP: f32 = 540.0;
pub const HEADER_HEIGHT: f32 = 52.0;

const FOOTER_HEIGHT: f32 = 48.0;
const OUTER_PADDING: f32 = 16.0;
const LIST_VERTICAL_PAD: f32 = 6.0;
const CARD_RADIUS: f32 = 8.0;
const CONTROL_RADIUS: f32 = 4.0;
const WINDOW_CORNER_RADIUS: f32 = 8.0;
const ROW_COLLAPSED_H: f32 = 66.0;
const ROW_EXPANDED_EXTRA_H: f32 = 106.0;
const SETTINGS_CARD_H: f32 = 56.0;
const SETTINGS_SUBROW_H: f32 = 48.0;
const SETTINGS_CARD_GAP: f32 = 4.0;

// Built-in Windows `Segoe Fluent Icons` / `Segoe MDL2 Assets` codepoints.
const GLYPH_PIN: &str = "\u{E718}";
const GLYPH_PINNED: &str = "\u{E840}";
const GLYPH_REFRESH: &str = "\u{E72C}";
const GLYPH_SETTINGS: &str = "\u{E713}";
const GLYPH_BACK: &str = "\u{E72B}";
const GLYPH_CHEVRON_DOWN: &str = "\u{E70D}";
const GLYPH_CHEVRON_UP: &str = "\u{E70E}";
const GLYPH_COPY: &str = "\u{E8C8}";
const GLYPH_CHECKMARK: &str = "\u{E73E}";
const GLYPH_COLOR_THEME: &str = "\u{E790}";
const GLYPH_LANGUAGE: &str = "\u{E774}";
const GLYPH_CLOCK: &str = "\u{E823}";
const GLYPH_RINGER: &str = "\u{EA8F}";
const GLYPH_RINGER_SILENT: &str = "\u{E7ED}";
const GLYPH_POWER: &str = "\u{E7E8}";
const GLYPH_MOUSE: &str = "\u{E962}";
const GLYPH_KEYBOARD: &str = "\u{E765}";
const GLYPH_GAMEPAD: &str = "\u{E7FC}";
const GLYPH_HEADSET: &str = "\u{E7F6}";
const GLYPH_PERIPHERAL: &str = "\u{E857}";
const GLYPH_CHARGING_BOLT: &str = "\u{E945}";

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
    OpenSettingsFromEmpty,
    SetPollInterval(u64),
    ToggleLowBatteryNotify,
    ToggleLowBatterySilent,
    ToggleAutostart,
}

#[derive(Debug, Clone)]
struct HitRegion {
    rect: D2D_RECT_F,
    target: HitTarget,
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
    fn settled(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            start: Instant::now(),
            duration_ms: 1,
            initialized: false,
        }
    }

    fn snap(&mut self, value: f32) {
        self.from = value;
        self.to = value;
        self.initialized = true;
    }

    fn animate_to(&mut self, target: f32, duration_ms: u32, now: Instant) {
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

    fn restart(&mut self, from: f32, to: f32, duration_ms: u32, now: Instant) {
        self.from = from;
        self.to = to;
        self.start = now;
        self.duration_ms = duration_ms.max(1);
        self.initialized = true;
    }

    fn value(&self, now: Instant) -> f32 {
        if (self.to - self.from).abs() < 1e-4 {
            return self.to;
        }
        let elapsed_ms = now.saturating_duration_since(self.start).as_secs_f32() * 1000.0;
        let t = (elapsed_ms / (self.duration_ms as f32)).clamp(0.0, 1.0);
        // WinUI 3 Fluent fast-out slow-in cubic deceleration curve
        let eased = 1.0 - (1.0 - t).powi(3);
        self.from + (self.to - self.from) * eased
    }

    fn is_active(&self, now: Instant) -> bool {
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
    // On-demand WinUI 3 micro-animations (VSync-paced)
    toggle_notify_anim: ScalarAnim,
    toggle_silent_anim: ScalarAnim,
    toggle_autostart_anim: ScalarAnim,
    dropdown_anim: ScalarAnim,
    drawer_reveal_anim: ScalarAnim,
    refresh_anim_start: Option<Instant>,
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

/// Target surface parameters passed to `PopoverRenderer::paint`.
#[derive(Debug, Clone, Copy)]
pub struct PaintSurface {
    pub hwnd: HWND,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub dpi: u32,
    pub system_theme: SystemTheme,
    pub effective_dark: bool,
    pub acrylic_active: bool,
}

struct TextFormats {
    title: IDWriteTextFormat,
    device_name: IDWriteTextFormat,
    value_large: IDWriteTextFormat,
    value_discrete: IDWriteTextFormat,
    body: IDWriteTextFormat,
    body_center: IDWriteTextFormat,
    caption: IDWriteTextFormat,
    caption_right: IDWriteTextFormat,
    caption_center: IDWriteTextFormat,
    mono_right: IDWriteTextFormat,
    icon_header: IDWriteTextFormat,
    icon_device: IDWriteTextFormat,
    icon_setting: IDWriteTextFormat,
    icon_small: IDWriteTextFormat,
    icon_chevron: IDWriteTextFormat,
}

pub struct PopoverRenderer {
    d2d: ID2D1Factory,
    formats: TextFormats,
    target: Option<ID2D1HwndRenderTarget>,
    hit_regions: Vec<HitRegion>,
    content_height: f32,
    viewport_height: f32,
}

impl PopoverRenderer {
    pub fn new() -> windows::core::Result<Self> {
        let d2d: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
        let dwrite: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };

        let icon_font = resolve_icon_font_family(&dwrite);
        let ui_text_font =
            resolve_font_family(&dwrite, w!("Segoe UI Variable Text"), w!("Segoe UI"));
        let ui_display_font =
            resolve_font_family(&dwrite, w!("Segoe UI Variable Display"), w!("Segoe UI"));
        let ui_small_font =
            resolve_font_family(&dwrite, w!("Segoe UI Variable Small"), w!("Segoe UI"));
        let mono_font = resolve_font_family(&dwrite, w!("Cascadia Mono"), w!("Consolas"));

        let formats = TextFormats {
            title: create_format(
                &dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                15.0,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            device_name: create_format(
                &dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                13.5,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
                true,
            )?,
            value_large: create_format(
                &dwrite,
                ui_display_font,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                15.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
                false,
            )?,
            value_discrete: create_format(
                &dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                13.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
                false,
            )?,
            body: create_format(
                &dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                12.5,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            body_center: create_format(
                &dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_MEDIUM,
                12.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            caption: create_format(
                &dwrite,
                ui_small_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.5,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            caption_right: create_format(
                &dwrite,
                ui_small_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.5,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            caption_center: create_format(
                &dwrite,
                ui_small_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                true,
                false,
            )?,
            mono_right: create_format(
                &dwrite,
                mono_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            icon_header: create_format(
                &dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                14.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            icon_device: create_format(
                &dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                17.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            icon_setting: create_format(
                &dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                15.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            icon_small: create_format(
                &dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                12.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            icon_chevron: create_format(
                &dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                10.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
        };

        Ok(Self {
            d2d,
            formats,
            target: None,
            hit_regions: Vec::new(),
            content_height: 0.0,
            viewport_height: 0.0,
        })
    }

    /// Compute the ideal logical height (DIPs) for the current snapshot and view.
    pub fn desired_height_dip(&self, snapshot: &TraySnapshot, ui: &UiState) -> f32 {
        match ui.view {
            PopoverView::Settings => {
                let subrow_extra = if snapshot.config.low_battery_notify {
                    SETTINGS_SUBROW_H
                } else {
                    0.0
                };
                HEADER_HEIGHT
                    + OUTER_PADDING * 2.0
                    + SETTINGS_CARD_H * 5.0
                    + SETTINGS_CARD_GAP * 4.0
                    + subrow_extra
            }
            PopoverView::Main => {
                if snapshot.devices.is_empty() {
                    return HEADER_HEIGHT + 204.0 + FOOTER_HEIGHT;
                }
                let compact = ui.is_compact_pinned(snapshot);
                let mut list_h = LIST_VERTICAL_PAD * 2.0;
                for dev in &snapshot.devices {
                    list_h += ROW_COLLAPSED_H;
                    if !compact && ui.expanded_device_id.as_deref() == Some(dev.id.as_str()) {
                        list_h += ROW_EXPANDED_EXTRA_H;
                    }
                }
                if compact {
                    list_h.clamp(78.0, POPOVER_MAX_HEIGHT_DIP - HEADER_HEIGHT - FOOTER_HEIGHT)
                } else {
                    (HEADER_HEIGHT + list_h + FOOTER_HEIGHT).clamp(180.0, POPOVER_MAX_HEIGHT_DIP)
                }
            }
        }
    }

    pub fn max_scroll(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    pub fn hit_test(&self, x_dip: f32, y_dip: f32) -> Option<HitTarget> {
        // Iterate in reverse so child buttons / dropdown overlays win over enclosing regions.
        for region in self.hit_regions.iter().rev() {
            if x_dip >= region.rect.left
                && x_dip <= region.rect.right
                && y_dip >= region.rect.top
                && y_dip <= region.rect.bottom
            {
                return Some(region.target.clone());
            }
        }
        None
    }

    /// Render the complete popover frame in DIP coordinates.
    pub fn paint(
        &mut self,
        surface: PaintSurface,
        snapshot: &TraySnapshot,
        ui: &mut UiState,
    ) -> windows::core::Result<()> {
        let rt = self.ensure_target(
            surface.hwnd,
            surface.pixel_width,
            surface.pixel_height,
            surface.dpi,
        )?;
        let scale = (surface.dpi as f32) / 96.0;
        let width = (surface.pixel_width as f32) / scale;
        let height = (surface.pixel_height as f32) / scale;
        let palette = Palette::for_theme(surface.effective_dark, surface.system_theme.accent_rgb);
        let lang = Lang::resolve(snapshot.config.language);
        let compact = ui.is_compact_pinned(snapshot);
        let now = Instant::now();

        self.hit_regions.clear();

        unsafe {
            rt.BeginDraw();
            rt.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            if !compact {
                let bg_color =
                    if surface.acrylic_active && surface.system_theme.transparency_enabled {
                        palette.backdrop_tint
                    } else {
                        palette.solid_fallback
                    };
                let bg_brush = rt.CreateSolidColorBrush(&bg_color, None)?;
                rt.FillRectangle(
                    &D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: width,
                        bottom: height,
                    },
                    &bg_brush,
                );
            }

            match ui.view {
                PopoverView::Main => {
                    self.draw_main_view(&rt, width, height, &palette, lang, snapshot, ui, now)?;
                }
                PopoverView::Settings => {
                    self.draw_settings_view(
                        &rt,
                        width,
                        height,
                        &palette,
                        lang,
                        surface.system_theme,
                        snapshot,
                        ui,
                        now,
                    )?;
                }
            }

            if let Some(msg) = &ui.feedback_banner {
                self.draw_toast(&rt, width, height, &palette, snapshot, ui, msg)?;
            }

            if !compact {
                let border_brush = rt.CreateSolidColorBrush(&palette.outer_border, None)?;
                rt.DrawRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F {
                            left: 0.5,
                            top: 0.5,
                            right: width - 0.5,
                            bottom: height - 0.5,
                        },
                        radiusX: WINDOW_CORNER_RADIUS - 0.5,
                        radiusY: WINDOW_CORNER_RADIUS - 0.5,
                    },
                    &border_brush,
                    1.0,
                    None,
                );
            }

            let end_result = rt.EndDraw(None, None);
            if end_result.is_err() {
                self.target = None;
            }
        }
        Ok(())
    }

    fn ensure_target(
        &mut self,
        hwnd: HWND,
        pixel_width: u32,
        pixel_height: u32,
        dpi: u32,
    ) -> windows::core::Result<ID2D1HwndRenderTarget> {
        let size = D2D_SIZE_U {
            width: pixel_width.max(1),
            height: pixel_height.max(1),
        };
        if let Some(existing) = &self.target {
            unsafe {
                existing.SetDpi(dpi as f32, dpi as f32);
                let _ = existing.Resize(&size);
            }
            return Ok(existing.clone());
        }

        let props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: dpi as f32,
            dpiY: dpi as f32,
            ..Default::default()
        };
        let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd,
            pixelSize: size,
            presentOptions: D2D1_PRESENT_OPTIONS_IMMEDIATELY,
        };
        let target = unsafe { self.d2d.CreateHwndRenderTarget(&props, &hwnd_props)? };
        unsafe {
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        }
        self.target = Some(target.clone());
        Ok(target)
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_main_view(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        width: f32,
        height: f32,
        palette: &Palette,
        lang: Lang,
        snapshot: &TraySnapshot,
        ui: &mut UiState,
        now: Instant,
    ) -> windows::core::Result<()> {
        let compact = ui.is_compact_pinned(snapshot);
        unsafe {
            let text_primary = rt.CreateSolidColorBrush(&palette.text_primary, None)?;
            let text_secondary = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;
            let divider_brush = rt.CreateSolidColorBrush(&palette.divider, None)?;

            if !compact {
                // --- Win11 Flyout Header (52 DIPs) ---
                draw_text(
                    rt,
                    "Wireless Battery",
                    &self.formats.title,
                    rect_xywh(OUTER_PADDING, 10.0, width - 136.0, 32.0),
                    &text_primary,
                );

                let btn_size = 32.0;
                let btn_y = (HEADER_HEIGHT - btn_size) * 0.5;
                let settings_rect = rect_xywh(width - 12.0 - btn_size, btn_y, btn_size, btn_size);
                let refresh_rect = rect_xywh(
                    settings_rect.left - 4.0 - btn_size,
                    btn_y,
                    btn_size,
                    btn_size,
                );
                let pin_rect = rect_xywh(
                    refresh_rect.left - 4.0 - btn_size,
                    btn_y,
                    btn_size,
                    btn_size,
                );

                self.draw_pin_button(rt, pin_rect, palette, ui)?;
                self.draw_subtle_icon_button(
                    rt,
                    refresh_rect,
                    GLYPH_REFRESH,
                    &HitTarget::RefreshAll,
                    false,
                    palette,
                    ui,
                )?;
                self.draw_subtle_icon_button(
                    rt,
                    settings_rect,
                    GLYPH_SETTINGS,
                    &HitTarget::OpenSettings,
                    false,
                    palette,
                    ui,
                )?;

                rt.DrawLine(
                    D2D_POINT_2F {
                        x: 0.0,
                        y: HEADER_HEIGHT - 0.5,
                    },
                    D2D_POINT_2F {
                        x: width,
                        y: HEADER_HEIGHT - 0.5,
                    },
                    &divider_brush,
                    1.0,
                    None,
                );

                // Animated indeterminate Win11 Accent progress bar while refreshing
                if snapshot.refreshing {
                    let elapsed = ui
                        .refresh_anim_start
                        .map_or(0.0, |s| now.saturating_duration_since(s).as_secs_f32());
                    let phase = (elapsed / 1.2).fract();
                    let eased = if phase < 0.5 {
                        2.0 * phase * phase
                    } else {
                        1.0 - (-2.0 * phase + 2.0).powi(2) * 0.5
                    };
                    let seg_w = width * 0.34;
                    let seg_x = -seg_w + (width + seg_w) * eased;
                    let accent_brush = rt.CreateSolidColorBrush(&palette.accent_fill, None)?;
                    rt.PushAxisAlignedClip(
                        &rect_xywh(0.0, HEADER_HEIGHT - 2.5, width, 3.0),
                        D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                    );
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: rect_xywh(seg_x, HEADER_HEIGHT - 2.0, seg_w, 2.0),
                            radiusX: 1.0,
                            radiusY: 1.0,
                        },
                        &accent_brush,
                    );
                    rt.PopAxisAlignedClip();
                }
            }

            // --- Scrollable Divider-Separated Device List Area ---
            let list_top = if compact { 0.0 } else { HEADER_HEIGHT };
            let list_bottom = if compact {
                height
            } else {
                height - FOOTER_HEIGHT
            };
            self.viewport_height = (list_bottom - list_top).max(1.0);

            rt.PushAxisAlignedClip(
                &D2D_RECT_F {
                    left: 0.0,
                    top: list_top,
                    right: width,
                    bottom: list_bottom,
                },
                D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            );

            if snapshot.devices.is_empty() {
                self.content_height = self.viewport_height;
                ui.scroll_offset = 0.0;
                self.draw_empty_state(
                    rt,
                    D2D_RECT_F {
                        left: 0.0,
                        top: list_top,
                        right: width,
                        bottom: list_bottom,
                    },
                    palette,
                    lang,
                    snapshot,
                    ui,
                )?;
            } else {
                let mut total_h = LIST_VERTICAL_PAD * 2.0;
                for dev in &snapshot.devices {
                    total_h += ROW_COLLAPSED_H;
                    if !compact && ui.expanded_device_id.as_deref() == Some(dev.id.as_str()) {
                        total_h += ROW_EXPANDED_EXTRA_H;
                    }
                }
                self.content_height = total_h;
                ui.scroll_offset = ui.scroll_offset.clamp(0.0, self.max_scroll());

                let has_scroll = self.max_scroll() > 0.5;
                let mut y = list_top + LIST_VERTICAL_PAD - ui.scroll_offset;
                let reveal = ui.drawer_reveal_anim.value(now);
                let row_div_color = if compact {
                    with_alpha(palette.divider, 1.4)
                } else {
                    palette.divider
                };
                let row_div_brush = rt.CreateSolidColorBrush(&row_div_color, None)?;

                for (idx, dev) in snapshot.devices.iter().enumerate() {
                    if idx > 0 && y >= list_top - 2.0 && y <= list_bottom + 2.0 {
                        rt.DrawLine(
                            D2D_POINT_2F {
                                x: OUTER_PADDING,
                                y: y + 0.5,
                            },
                            D2D_POINT_2F {
                                x: width - OUTER_PADDING,
                                y: y + 0.5,
                            },
                            &row_div_brush,
                            1.0,
                            None,
                        );
                    }

                    let expanded =
                        !compact && ui.expanded_device_id.as_deref() == Some(dev.id.as_str());
                    let row_h = if expanded {
                        ROW_COLLAPSED_H + ROW_EXPANDED_EXTRA_H
                    } else {
                        ROW_COLLAPSED_H
                    };
                    let row_rect = rect_xywh(0.0, y, width, row_h);
                    if row_rect.bottom >= list_top - 4.0 && row_rect.top <= list_bottom + 4.0 {
                        self.draw_device_row(
                            rt, row_rect, dev, expanded, compact, reveal, palette, lang, ui,
                        )?;
                    }
                    y += row_h;
                }

                if has_scroll && !compact {
                    let track_top = list_top + 10.0;
                    let track_h = (list_bottom - list_top - 20.0).max(20.0);
                    let thumb_h = ((self.viewport_height / self.content_height) * track_h)
                        .clamp(28.0, track_h);
                    let scroll_ratio = ui.scroll_offset / self.max_scroll();
                    let thumb_y = track_top + scroll_ratio * (track_h - thumb_h);
                    let sb_brush = rt.CreateSolidColorBrush(&palette.scrollbar_thumb, None)?;
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: rect_xywh(width - 6.0, thumb_y, 3.0, thumb_h),
                            radiusX: 1.5,
                            radiusY: 1.5,
                        },
                        &sb_brush,
                    );
                }
            }

            rt.PopAxisAlignedClip();

            if !compact {
                // --- Minimalist Win11 Quick Settings Bottom Action Bar (48 DIPs) ---
                rt.PushAxisAlignedClip(
                    &D2D_RECT_F {
                        left: 0.0,
                        top: list_bottom,
                        right: width,
                        bottom: height,
                    },
                    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                );
                let footer_bg = rt.CreateSolidColorBrush(&palette.footer_bg, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F {
                            left: 0.0,
                            top: list_bottom - WINDOW_CORNER_RADIUS,
                            right: width,
                            bottom: height,
                        },
                        radiusX: WINDOW_CORNER_RADIUS,
                        radiusY: WINDOW_CORNER_RADIUS,
                    },
                    &footer_bg,
                );
                rt.PopAxisAlignedClip();

                rt.DrawLine(
                    D2D_POINT_2F {
                        x: 0.0,
                        y: list_bottom + 0.5,
                    },
                    D2D_POINT_2F {
                        x: width,
                        y: list_bottom + 0.5,
                    },
                    &divider_brush,
                    1.0,
                    None,
                );

                let dot_color = match snapshot.icon_state {
                    TrayIconState::Idle => palette.text_subtle,
                    TrayIconState::LowBattery => palette.status_critical,
                    TrayIconState::Normal => {
                        let all_stale = !snapshot.devices.is_empty()
                            && snapshot
                                .devices
                                .iter()
                                .all(|d| d.freshness == Some(Freshness::Stale));
                        if all_stale {
                            palette.status_caution
                        } else {
                            palette.status_good
                        }
                    }
                };
                let dot_brush = rt.CreateSolidColorBrush(&dot_color, None)?;
                let footer_mid_y = list_bottom + FOOTER_HEIGHT * 0.5;
                rt.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: D2D_POINT_2F {
                            x: OUTER_PADDING + 3.0,
                            y: footer_mid_y,
                        },
                        radiusX: 3.0,
                        radiusY: 3.0,
                    },
                    &dot_brush,
                );

                let footer_status = if snapshot.refreshing {
                    lang.reading_peripherals().to_owned()
                } else {
                    lang.footer_summary(snapshot.last_sync_at, snapshot.config.poll_interval_secs)
                };
                draw_text(
                    rt,
                    &footer_status,
                    &self.formats.caption,
                    rect_xywh(
                        OUTER_PADDING + 13.0,
                        list_bottom,
                        width - 96.0,
                        FOOTER_HEIGHT,
                    ),
                    &text_secondary,
                );

                let quit_rect = rect_xywh(width - 12.0 - 48.0, list_bottom + 8.0, 48.0, 32.0);
                let quit_hovered = ui.hovered == Some(HitTarget::Quit);
                let quit_pressed = ui.pressed == Some(HitTarget::Quit);
                if quit_pressed || quit_hovered {
                    let fill = if quit_pressed {
                        &palette.subtle_pressed
                    } else {
                        &palette.subtle_hover
                    };
                    let hb = rt.CreateSolidColorBrush(fill, None)?;
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: quit_rect,
                            radiusX: CONTROL_RADIUS,
                            radiusY: CONTROL_RADIUS,
                        },
                        &hb,
                    );
                }
                draw_text(
                    rt,
                    lang.quit(),
                    &self.formats.body_center,
                    quit_rect,
                    if quit_hovered || quit_pressed {
                        &text_primary
                    } else {
                        &text_secondary
                    },
                );
                self.register_hit(quit_rect, HitTarget::Quit);
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_empty_state(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        bounds: D2D_RECT_F,
        palette: &Palette,
        lang: Lang,
        snapshot: &TraySnapshot,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        let width = bounds.right - bounds.left;
        let top = bounds.top;
        let bottom = bounds.bottom;
        unsafe {
            let center_y = top + (bottom - top) * 0.5;
            let well_rect = rect_xywh((width - 44.0) * 0.5, center_y - 70.0, 44.0, 44.0);
            let well_brush = rt.CreateSolidColorBrush(&palette.icon_well, None)?;
            let glyph_brush = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;
            let text_primary = rt.CreateSolidColorBrush(&palette.text_primary, None)?;
            let text_muted = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;

            rt.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: well_rect,
                    radiusX: CARD_RADIUS,
                    radiusY: CARD_RADIUS,
                },
                &well_brush,
            );
            draw_text(
                rt,
                GLYPH_PERIPHERAL,
                &self.formats.icon_device,
                well_rect,
                &glyph_brush,
            );

            draw_text(
                rt,
                lang.empty_title(),
                &self.formats.body_center,
                rect_xywh(24.0, center_y - 16.0, width - 48.0, 22.0),
                &text_primary,
            );

            let hint = lang.empty_hint(snapshot.hidden_experimental_count);
            draw_text(
                rt,
                &hint,
                &self.formats.caption_center,
                rect_xywh(36.0, center_y + 10.0, width - 72.0, 36.0),
                &text_muted,
            );

            let has_hidden = snapshot.hidden_experimental_count > 0;
            let btn_w = if has_hidden { 132.0 } else { 112.0 };
            let btn_rect = rect_xywh((width - btn_w) * 0.5, center_y + 54.0, btn_w, 30.0);
            let target = if has_hidden {
                HitTarget::OpenSettingsFromEmpty
            } else {
                HitTarget::RefreshAll
            };
            self.draw_standard_button(
                rt,
                btn_rect,
                None,
                lang.empty_action(has_hidden),
                target,
                palette,
                ui,
                1.0,
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_device_row(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        rect: D2D_RECT_F,
        dev: &DeviceView,
        expanded: bool,
        compact: bool,
        drawer_alpha: f32,
        palette: &Palette,
        lang: Lang,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        unsafe {
            let toggle_target = HitTarget::ToggleDevice(dev.id.clone());
            let hovered = !compact && ui.hovered.as_ref() == Some(&toggle_target);
            let pressed = !compact && ui.pressed.as_ref() == Some(&toggle_target);

            if !compact && (hovered || pressed) {
                let header_hit_rect = rect_xywh(
                    rect.left + 8.0,
                    rect.top + 3.0,
                    (rect.right - rect.left) - 16.0,
                    ROW_COLLAPSED_H - 6.0,
                );
                let fill = if pressed {
                    &palette.subtle_pressed
                } else {
                    &palette.subtle_hover
                };
                let fill_brush = rt.CreateSolidColorBrush(fill, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: header_hit_rect,
                        radiusX: 6.0,
                        radiusY: 6.0,
                    },
                    &fill_brush,
                );
            }

            if !compact {
                self.register_hit(
                    rect_xywh(
                        rect.left + 8.0,
                        rect.top,
                        (rect.right - rect.left) - 16.0,
                        ROW_COLLAPSED_H,
                    ),
                    toggle_target,
                );
            }

            let shadow_brush = if compact {
                Some(rt.CreateSolidColorBrush(&palette.compact_text_shadow, None)?)
            } else {
                None
            };

            let well_rect = rect_xywh(rect.left + OUTER_PADDING, rect.top + 15.0, 36.0, 36.0);
            let glyph_brush = rt.CreateSolidColorBrush(&palette.icon_stroke, None)?;
            if !compact {
                let well_brush = rt.CreateSolidColorBrush(&palette.icon_well, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: well_rect,
                        radiusX: CARD_RADIUS,
                        radiusY: CARD_RADIUS,
                    },
                    &well_brush,
                );
            }
            draw_text_shadowed(
                rt,
                device_kind_glyph(dev.kind),
                &self.formats.icon_device,
                well_rect,
                &glyph_brush,
                shadow_brush.as_ref(),
            );

            let is_stale = dev.freshness == Some(Freshness::Stale);
            let is_offline = dev.reachability == Reachability::Offline;

            let name_brush = rt.CreateSolidColorBrush(
                if is_offline {
                    &palette.text_secondary
                } else {
                    &palette.text_primary
                },
                None,
            )?;
            let sub_color = if dev.charge_state == ChargeState::Charging {
                palette.status_good
            } else if dev.precision == Precision::None && dev.last_error.is_some() {
                palette.status_caution
            } else {
                palette.text_secondary
            };
            let sub_brush = rt.CreateSolidColorBrush(&sub_color, None)?;

            let text_left = rect.left + OUTER_PADDING + 36.0 + 12.0;
            let right_w = 68.0;
            let right_x = if compact {
                rect.right - OUTER_PADDING - right_w
            } else {
                rect.right - OUTER_PADDING - 22.0 - 8.0 - right_w
            };
            let text_width = (right_x - text_left - 10.0).max(40.0);

            draw_text_shadowed(
                rt,
                &dev.name,
                &self.formats.device_name,
                rect_xywh(text_left, rect.top + 14.0, text_width, 20.0),
                &name_brush,
                shadow_brush.as_ref(),
            );

            let subtitle = lang.format_device_subtitle(dev);
            draw_text_shadowed(
                rt,
                &subtitle,
                &self.formats.caption,
                rect_xywh(text_left, rect.top + 35.0, text_width, 18.0),
                &sub_brush,
                shadow_brush.as_ref(),
            );

            let bar_y = rect.top + 42.0;
            let track_brush = rt.CreateSolidColorBrush(&palette.bar_track, None)?;

            let (val_color, bar_color) = battery_tone(dev, palette);
            let val_brush = rt.CreateSolidColorBrush(&val_color, None)?;
            let fill_brush = rt.CreateSolidColorBrush(&bar_color, None)?;

            match dev.precision {
                Precision::Percentage => {
                    if let Some(pct) = dev.percentage {
                        if dev.charge_state == ChargeState::Charging {
                            let bolt_brush =
                                rt.CreateSolidColorBrush(&palette.status_good, None)?;
                            let num_w = if pct >= 100 {
                                41.0
                            } else if pct >= 10 {
                                32.0
                            } else {
                                23.0
                            };
                            let bolt_x = (right_x + right_w - num_w - 13.0).max(right_x - 2.0);
                            draw_text_shadowed(
                                rt,
                                GLYPH_CHARGING_BOLT,
                                &self.formats.icon_small,
                                rect_xywh(bolt_x, rect.top + 16.0, 12.0, 16.0),
                                &bolt_brush,
                                shadow_brush.as_ref(),
                            );
                        }
                        let label = format!("{pct}%");
                        draw_text_shadowed(
                            rt,
                            &label,
                            &self.formats.value_large,
                            rect_xywh(right_x, rect.top + 14.0, right_w, 21.0),
                            &val_brush,
                            shadow_brush.as_ref(),
                        );
                        let track_rect = rect_xywh(right_x, bar_y, right_w, 4.0);
                        rt.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: track_rect,
                                radiusX: 2.0,
                                radiusY: 2.0,
                            },
                            &track_brush,
                        );
                        let fill_w = (right_w * (pct as f32 / 100.0)).clamp(4.0, right_w);
                        rt.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: rect_xywh(right_x, bar_y, fill_w, 4.0),
                                radiusX: 2.0,
                                radiusY: 2.0,
                            },
                            &fill_brush,
                        );
                    }
                }
                Precision::DiscreteLevel => {
                    let (segments, label) = lang.discrete_info(dev.level);
                    draw_text_shadowed(
                        rt,
                        label,
                        &self.formats.value_discrete,
                        rect_xywh(right_x, rect.top + 16.0, right_w, 19.0),
                        &val_brush,
                        shadow_brush.as_ref(),
                    );
                    let gap = 3.0;
                    let seg_w = (right_w - gap * 3.0) / 4.0;
                    for i in 0..4u8 {
                        let sx = right_x + (i as f32) * (seg_w + gap);
                        let seg_rect = rect_xywh(sx, bar_y, seg_w, 4.0);
                        let b = if i < segments {
                            &fill_brush
                        } else {
                            &track_brush
                        };
                        rt.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: seg_rect,
                                radiusX: 2.0,
                                radiusY: 2.0,
                            },
                            b,
                        );
                    }
                }
                Precision::VoltageOnly => {
                    let label = dev
                        .voltage_mv
                        .map(|mv| format!("{:.2} V", f64::from(mv) / 1000.0))
                        .unwrap_or_else(|| "--".to_owned());
                    draw_text_shadowed(
                        rt,
                        &label,
                        &self.formats.value_discrete,
                        rect_xywh(right_x, rect.top + 16.0, right_w, 19.0),
                        &val_brush,
                        shadow_brush.as_ref(),
                    );
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: rect_xywh(right_x, bar_y, right_w, 4.0),
                            radiusX: 2.0,
                            radiusY: 2.0,
                        },
                        &track_brush,
                    );
                }
                Precision::None => {
                    let subtle_brush = rt.CreateSolidColorBrush(&palette.text_subtle, None)?;
                    draw_text_shadowed(
                        rt,
                        "--",
                        &self.formats.value_large,
                        rect_xywh(right_x, rect.top + 14.0, right_w, 21.0),
                        &subtle_brush,
                        shadow_brush.as_ref(),
                    );
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: rect_xywh(right_x, bar_y, right_w, 4.0),
                            radiusX: 2.0,
                            radiusY: 2.0,
                        },
                        &track_brush,
                    );
                }
            }

            if !compact {
                let chevron_brush = rt.CreateSolidColorBrush(
                    if hovered || expanded {
                        &palette.text_secondary
                    } else {
                        &palette.text_subtle
                    },
                    None,
                )?;
                draw_text(
                    rt,
                    if expanded {
                        GLYPH_CHEVRON_UP
                    } else {
                        GLYPH_CHEVRON_DOWN
                    },
                    &self.formats.icon_chevron,
                    rect_xywh(
                        rect.right - OUTER_PADDING - 20.0,
                        rect.top + 22.0,
                        20.0,
                        22.0,
                    ),
                    &chevron_brush,
                );
            }

            // Expanded detail drawer (Protocol, Bus, Status — no Evidence; "Copy error" only on error)
            if expanded {
                let drawer_top = rect.top + ROW_COLLAPSED_H + 2.0;
                let drawer_rect = rect_xywh(
                    rect.left + OUTER_PADDING,
                    drawer_top,
                    (rect.right - rect.left) - OUTER_PADDING * 2.0,
                    ROW_EXPANDED_EXTRA_H - 12.0,
                );
                let drawer_bg = rt.CreateSolidColorBrush(
                    &with_alpha(palette.card_drawer_bg, drawer_alpha),
                    None,
                )?;
                let drawer_border =
                    rt.CreateSolidColorBrush(&with_alpha(palette.card_border, drawer_alpha), None)?;
                let drawer_rr = D2D1_ROUNDED_RECT {
                    rect: drawer_rect,
                    radiusX: 6.0,
                    radiusY: 6.0,
                };
                rt.FillRoundedRectangle(&drawer_rr, &drawer_bg);
                rt.DrawRoundedRectangle(&drawer_rr, &drawer_border, 1.0, None);

                let subtle_brush =
                    rt.CreateSolidColorBrush(&with_alpha(palette.text_subtle, drawer_alpha), None)?;
                let sec_brush = rt.CreateSolidColorBrush(
                    &with_alpha(palette.text_secondary, drawer_alpha),
                    None,
                )?;

                let slide_y = (1.0 - drawer_alpha) * -4.0;
                let pad_x = 12.0;
                let inner_w = (drawer_rect.right - drawer_rect.left) - pad_x * 2.0;
                let col_w = (inner_w - 16.0) * 0.5;
                let col1_x = drawer_rect.left + pad_x;
                let col2_x = col1_x + col_w + 16.0;
                let row1_y = drawer_rect.top + 10.0 + slide_y;
                let row2_y = row1_y + 21.0;

                // Row 1: Protocol & Bus
                let proto_label_w = if lang.is_zh() { 34.0 } else { 52.0 };
                draw_text(
                    rt,
                    lang.label_protocol(),
                    &self.formats.caption,
                    rect_xywh(col1_x, row1_y, proto_label_w, 18.0),
                    &subtle_brush,
                );
                draw_text(
                    rt,
                    dev.provider_id.as_deref().unwrap_or("--"),
                    &self.formats.mono_right,
                    rect_xywh(
                        col1_x + proto_label_w,
                        row1_y,
                        (col_w - proto_label_w).max(40.0),
                        18.0,
                    ),
                    &sec_brush,
                );

                let bus_label_w = if lang.is_zh() { 34.0 } else { 32.0 };
                draw_text(
                    rt,
                    lang.label_bus(),
                    &self.formats.caption,
                    rect_xywh(col2_x, row1_y, bus_label_w, 18.0),
                    &subtle_brush,
                );
                draw_text(
                    rt,
                    &dev.transport_label,
                    &self.formats.caption_right,
                    rect_xywh(
                        col2_x + bus_label_w,
                        row1_y,
                        (col_w - bus_label_w).max(40.0),
                        18.0,
                    ),
                    &sec_brush,
                );

                // Row 2: Status
                let status_label_w = if lang.is_zh() { 34.0 } else { 44.0 };
                draw_text(
                    rt,
                    lang.label_status(),
                    &self.formats.caption,
                    rect_xywh(col1_x, row2_y, status_label_w, 18.0),
                    &subtle_brush,
                );
                let (status_text, status_color) = if let Some(err) = dev.last_error {
                    (lang.error_kind_short_label(err), palette.status_caution)
                } else if is_stale {
                    (lang.status_stale(), palette.text_secondary)
                } else if dev.freshness == Some(Freshness::Live) {
                    (lang.status_live(), palette.status_good)
                } else {
                    (lang.status_unread(), palette.text_secondary)
                };
                let status_brush =
                    rt.CreateSolidColorBrush(&with_alpha(status_color, drawer_alpha), None)?;
                draw_text(
                    rt,
                    status_text,
                    &self.formats.caption_right,
                    rect_xywh(
                        col1_x + status_label_w,
                        row2_y,
                        (inner_w - status_label_w).max(60.0),
                        18.0,
                    ),
                    &status_brush,
                );

                // Bottom action row: always "Refresh"; show "Copy error" ONLY when read error exists
                let btn_y = row2_y + 25.0;
                let has_read_error = dev.last_error.is_some();
                let read_w = if lang.is_zh() { 86.0 } else { 84.0 };
                let read_target = HitTarget::RefreshDevice(dev.id.clone());

                if has_read_error {
                    let err_w = if lang.is_zh() { 88.0 } else { 96.0 };
                    let err_rect = rect_xywh(drawer_rect.right - pad_x - err_w, btn_y, err_w, 28.0);
                    let read_rect = rect_xywh(err_rect.left - 8.0 - read_w, btn_y, read_w, 28.0);
                    self.draw_standard_button(
                        rt,
                        read_rect,
                        Some(GLYPH_REFRESH),
                        lang.btn_refresh_device(),
                        read_target,
                        palette,
                        ui,
                        drawer_alpha,
                    )?;
                    self.draw_standard_button(
                        rt,
                        err_rect,
                        Some(GLYPH_COPY),
                        lang.btn_copy_error(),
                        HitTarget::CopyError(dev.id.clone()),
                        palette,
                        ui,
                        drawer_alpha,
                    )?;
                } else {
                    let read_rect =
                        rect_xywh(drawer_rect.right - pad_x - read_w, btn_y, read_w, 28.0);
                    self.draw_standard_button(
                        rt,
                        read_rect,
                        Some(GLYPH_REFRESH),
                        lang.btn_refresh_device(),
                        read_target,
                        palette,
                        ui,
                        drawer_alpha,
                    )?;
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_settings_view(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        width: f32,
        height: f32,
        palette: &Palette,
        lang: Lang,
        system_theme: SystemTheme,
        snapshot: &TraySnapshot,
        ui: &UiState,
        now: Instant,
    ) -> windows::core::Result<()> {
        unsafe {
            let text_primary = rt.CreateSolidColorBrush(&palette.text_primary, None)?;
            let text_secondary = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;
            let icon_brush = rt.CreateSolidColorBrush(&palette.icon_stroke, None)?;
            let divider_brush = rt.CreateSolidColorBrush(&palette.divider, None)?;

            // --- Win11 Settings Header (52 DIPs) ---
            let back_rect = rect_xywh(12.0, 10.0, 32.0, 32.0);
            self.draw_subtle_icon_button(
                rt,
                back_rect,
                GLYPH_BACK,
                &HitTarget::BackToMain,
                false,
                palette,
                ui,
            )?;

            draw_text(
                rt,
                lang.settings_title(),
                &self.formats.title,
                rect_xywh(50.0, 10.0, width - 66.0, 32.0),
                &text_primary,
            );

            rt.DrawLine(
                D2D_POINT_2F {
                    x: 0.0,
                    y: HEADER_HEIGHT - 0.5,
                },
                D2D_POINT_2F {
                    x: width,
                    y: HEADER_HEIGHT - 0.5,
                },
                &divider_brush,
                1.0,
                None,
            );

            // --- Flat Independent WinUI 3 SettingsCards (4px gap) ---
            let pad_x = OUTER_PADDING;
            let inner_w = width - pad_x * 2.0;
            let mut y = HEADER_HEIGHT + OUTER_PADDING;
            let combo_w = 104.0;
            let combo_h = 32.0;

            // Card 1: Theme (ComboBox)
            let theme_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
            let theme_combo_rect = rect_xywh(
                theme_card.right - 14.0 - combo_w,
                theme_card.top + (SETTINGS_CARD_H - combo_h) * 0.5,
                combo_w,
                combo_h,
            );
            draw_settings_card_base(
                rt,
                theme_card,
                GLYPH_COLOR_THEME,
                lang.setting_theme_title(),
                lang.setting_theme_desc(snapshot.config.theme_mode, system_theme.app_dark),
                combo_w + 22.0,
                &self.formats,
                &icon_brush,
                &text_primary,
                &text_secondary,
                palette,
                false,
                false,
            )?;
            self.draw_combobox_button(
                rt,
                theme_combo_rect,
                lang.theme_mode_label(snapshot.config.theme_mode),
                OpenDropdown::ThemeMode,
                palette,
                ui,
            )?;
            y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

            // Card 2: Language (ComboBox)
            let lang_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
            let lang_combo_rect = rect_xywh(
                lang_card.right - 14.0 - combo_w,
                lang_card.top + (SETTINGS_CARD_H - combo_h) * 0.5,
                combo_w,
                combo_h,
            );
            draw_settings_card_base(
                rt,
                lang_card,
                GLYPH_LANGUAGE,
                lang.setting_language_title(),
                lang.setting_language_desc(snapshot.config.language),
                combo_w + 22.0,
                &self.formats,
                &icon_brush,
                &text_primary,
                &text_secondary,
                palette,
                false,
                false,
            )?;
            self.draw_combobox_button(
                rt,
                lang_combo_rect,
                lang.language_mode_label(snapshot.config.language),
                OpenDropdown::Language,
                palette,
                ui,
            )?;
            y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

            // Card 3: Poll Interval (ComboBox — includes 1 hour option)
            let poll_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
            let poll_combo_rect = rect_xywh(
                poll_card.right - 14.0 - combo_w,
                poll_card.top + (SETTINGS_CARD_H - combo_h) * 0.5,
                combo_w,
                combo_h,
            );
            draw_settings_card_base(
                rt,
                poll_card,
                GLYPH_CLOCK,
                lang.setting_poll_title(),
                lang.setting_poll_desc(),
                combo_w + 22.0,
                &self.formats,
                &icon_brush,
                &text_primary,
                &text_secondary,
                palette,
                false,
                false,
            )?;
            self.draw_combobox_button(
                rt,
                poll_combo_rect,
                lang.poll_interval_label(snapshot.config.poll_interval_secs),
                OpenDropdown::PollInterval,
                palette,
                ui,
            )?;
            y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

            // Card 4: Low Battery Alert + nested Silent Notifications sub-option
            let show_subrow = snapshot.config.low_battery_notify;
            let notify_card_h = if show_subrow {
                SETTINGS_CARD_H + SETTINGS_SUBROW_H
            } else {
                SETTINGS_CARD_H
            };
            let notify_card = rect_xywh(pad_x, y, inner_w, notify_card_h);
            let notify_header_rect = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
            let notify_target = HitTarget::ToggleLowBatteryNotify;
            let notify_hovered =
                ui.open_dropdown.is_none() && ui.hovered.as_ref() == Some(&notify_target);
            let notify_pressed =
                ui.open_dropdown.is_none() && ui.pressed.as_ref() == Some(&notify_target);

            let card_bg = if !show_subrow && notify_pressed {
                palette.card_bg_pressed
            } else if !show_subrow && notify_hovered {
                palette.card_bg_hover
            } else {
                palette.card_bg
            };
            draw_elevated_card(
                rt,
                notify_card,
                CARD_RADIUS,
                card_bg,
                palette.card_border,
                palette,
            )?;

            // Highlight top row only when sub-row is visible and top row is hovered/pressed
            if show_subrow && (notify_hovered || notify_pressed) {
                let row_fill = if notify_pressed {
                    &palette.subtle_pressed
                } else {
                    &palette.subtle_hover
                };
                let rb = rt.CreateSolidColorBrush(row_fill, None)?;
                rt.PushAxisAlignedClip(
                    &D2D_RECT_F {
                        left: notify_card.left + 1.0,
                        top: notify_card.top + 1.0,
                        right: notify_card.right - 1.0,
                        bottom: notify_card.top + SETTINGS_CARD_H,
                    },
                    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                );
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F {
                            left: notify_card.left + 1.0,
                            top: notify_card.top + 1.0,
                            right: notify_card.right - 1.0,
                            bottom: notify_card.bottom - 1.0,
                        },
                        radiusX: CARD_RADIUS - 1.0,
                        radiusY: CARD_RADIUS - 1.0,
                    },
                    &rb,
                );
                rt.PopAxisAlignedClip();
            }

            draw_settings_row_content(
                rt,
                notify_header_rect,
                GLYPH_RINGER,
                lang.setting_low_battery_title(),
                lang.setting_low_battery_desc(
                    snapshot.config.low_battery_notify,
                    snapshot.config.low_battery_silent,
                ),
                64.0,
                &self.formats,
                &icon_brush,
                &text_primary,
                &text_secondary,
            );
            draw_win11_toggle_switch(
                rt,
                notify_header_rect.right - 14.0 - 40.0,
                notify_header_rect.top + (SETTINGS_CARD_H - 20.0) * 0.5,
                ui.toggle_notify_anim.value(now),
                notify_hovered,
                notify_pressed,
                palette,
            )?;
            self.register_hit(notify_header_rect, notify_target);

            if show_subrow {
                let sub_top = notify_card.top + SETTINGS_CARD_H;
                let sub_rect = rect_xywh(
                    notify_card.left + 1.0,
                    sub_top,
                    (notify_card.right - notify_card.left) - 2.0,
                    SETTINGS_SUBROW_H - 1.0,
                );
                let silent_target = HitTarget::ToggleLowBatterySilent;
                let silent_hovered =
                    ui.open_dropdown.is_none() && ui.hovered.as_ref() == Some(&silent_target);
                let silent_pressed =
                    ui.open_dropdown.is_none() && ui.pressed.as_ref() == Some(&silent_target);

                // Recessed WinUI 3 SettingsExpander sub-item band clipped to bottom card corners
                rt.PushAxisAlignedClip(&sub_rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                let sub_bg_color = if silent_pressed {
                    palette.card_bg_pressed
                } else if silent_hovered {
                    palette.card_bg_hover
                } else {
                    palette.card_drawer_bg
                };
                let sub_bg = rt.CreateSolidColorBrush(&sub_bg_color, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F {
                            left: notify_card.left + 0.5,
                            top: notify_card.top + 0.5,
                            right: notify_card.right - 0.5,
                            bottom: notify_card.bottom - 0.5,
                        },
                        radiusX: CARD_RADIUS - 0.5,
                        radiusY: CARD_RADIUS - 0.5,
                    },
                    &sub_bg,
                );
                rt.PopAxisAlignedClip();

                rt.DrawLine(
                    D2D_POINT_2F {
                        x: notify_card.left + 1.0,
                        y: sub_top + 0.5,
                    },
                    D2D_POINT_2F {
                        x: notify_card.right - 1.0,
                        y: sub_top + 0.5,
                    },
                    &divider_brush,
                    1.0,
                    None,
                );

                // Indented sub-option icon + title + description + ToggleSwitch
                let sub_icon_brush = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;
                draw_text(
                    rt,
                    GLYPH_RINGER_SILENT,
                    &self.formats.icon_small,
                    rect_xywh(notify_card.left + 26.0, sub_top, 18.0, SETTINGS_SUBROW_H),
                    &sub_icon_brush,
                );
                let sub_text_x = notify_card.left + 48.0;
                let sub_text_w = ((notify_card.right - notify_card.left) - 48.0 - 64.0).max(60.0);
                draw_text(
                    rt,
                    lang.setting_silent_title(),
                    &self.formats.body,
                    rect_xywh(sub_text_x, sub_top + 7.0, sub_text_w, 18.0),
                    &text_primary,
                );
                draw_text(
                    rt,
                    lang.setting_silent_desc(),
                    &self.formats.caption,
                    rect_xywh(sub_text_x, sub_top + 25.0, sub_text_w, 16.0),
                    &text_secondary,
                );
                draw_win11_toggle_switch(
                    rt,
                    notify_card.right - 14.0 - 40.0,
                    sub_top + (SETTINGS_SUBROW_H - 20.0) * 0.5,
                    ui.toggle_silent_anim.value(now),
                    silent_hovered,
                    silent_pressed,
                    palette,
                )?;
                self.register_hit(sub_rect, silent_target);
            }
            y += notify_card_h + SETTINGS_CARD_GAP;

            // Card 5: Launch at Startup (ToggleSwitch)
            let auto_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
            let auto_target = HitTarget::ToggleAutostart;
            let auto_hovered =
                ui.open_dropdown.is_none() && ui.hovered.as_ref() == Some(&auto_target);
            let auto_pressed =
                ui.open_dropdown.is_none() && ui.pressed.as_ref() == Some(&auto_target);
            draw_settings_card_base(
                rt,
                auto_card,
                GLYPH_POWER,
                lang.setting_autostart_title(),
                lang.setting_autostart_desc(),
                64.0,
                &self.formats,
                &icon_brush,
                &text_primary,
                &text_secondary,
                palette,
                auto_hovered,
                auto_pressed,
            )?;
            draw_win11_toggle_switch(
                rt,
                auto_card.right - 14.0 - 40.0,
                auto_card.top + (SETTINGS_CARD_H - 20.0) * 0.5,
                ui.toggle_autostart_anim.value(now),
                auto_hovered,
                auto_pressed,
                palette,
            )?;
            self.register_hit(auto_card, auto_target);

            // --- WinUI 3 ComboBox Dropdown Flyout Overlay (if open) ---
            if let Some(open) = ui.open_dropdown {
                self.register_hit(rect_xywh(0.0, 0.0, width, height), HitTarget::CloseDropdown);
                let anim = ui.dropdown_anim.value(now);
                match open {
                    OpenDropdown::ThemeMode => {
                        let options = [
                            (
                                HitTarget::SetThemeMode(ThemeMode::System),
                                lang.theme_mode_label(ThemeMode::System),
                                snapshot.config.theme_mode == ThemeMode::System,
                            ),
                            (
                                HitTarget::SetThemeMode(ThemeMode::Light),
                                lang.theme_mode_label(ThemeMode::Light),
                                snapshot.config.theme_mode == ThemeMode::Light,
                            ),
                            (
                                HitTarget::SetThemeMode(ThemeMode::Dark),
                                lang.theme_mode_label(ThemeMode::Dark),
                                snapshot.config.theme_mode == ThemeMode::Dark,
                            ),
                        ];
                        self.draw_dropdown_flyout(
                            rt,
                            theme_combo_rect,
                            height,
                            &options,
                            anim,
                            palette,
                            ui,
                        )?;
                    }
                    OpenDropdown::Language => {
                        let options = [
                            (
                                HitTarget::SetLanguageMode(LanguageMode::System),
                                lang.language_mode_label(LanguageMode::System),
                                snapshot.config.language == LanguageMode::System,
                            ),
                            (
                                HitTarget::SetLanguageMode(LanguageMode::ZhCn),
                                lang.language_mode_label(LanguageMode::ZhCn),
                                snapshot.config.language == LanguageMode::ZhCn,
                            ),
                            (
                                HitTarget::SetLanguageMode(LanguageMode::En),
                                lang.language_mode_label(LanguageMode::En),
                                snapshot.config.language == LanguageMode::En,
                            ),
                        ];
                        self.draw_dropdown_flyout(
                            rt,
                            lang_combo_rect,
                            height,
                            &options,
                            anim,
                            palette,
                            ui,
                        )?;
                    }
                    OpenDropdown::PollInterval => {
                        let options = [
                            (
                                HitTarget::SetPollInterval(300),
                                lang.poll_interval_label(300),
                                snapshot.config.poll_interval_secs == 300,
                            ),
                            (
                                HitTarget::SetPollInterval(900),
                                lang.poll_interval_label(900),
                                snapshot.config.poll_interval_secs == 900,
                            ),
                            (
                                HitTarget::SetPollInterval(1800),
                                lang.poll_interval_label(1800),
                                snapshot.config.poll_interval_secs == 1800,
                            ),
                            (
                                HitTarget::SetPollInterval(3600),
                                lang.poll_interval_label(3600),
                                snapshot.config.poll_interval_secs == 3600,
                            ),
                            (
                                HitTarget::SetPollInterval(0),
                                lang.poll_interval_label(0),
                                snapshot.config.poll_interval_secs == 0,
                            ),
                        ];
                        self.draw_dropdown_flyout(
                            rt,
                            poll_combo_rect,
                            height,
                            &options,
                            anim,
                            palette,
                            ui,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn draw_combobox_button(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        rect: D2D_RECT_F,
        label: &str,
        dropdown: OpenDropdown,
        palette: &Palette,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        let target = HitTarget::ToggleDropdown(dropdown);
        let is_open = ui.open_dropdown == Some(dropdown);
        let hovered = ui.open_dropdown.is_none() && ui.hovered.as_ref() == Some(&target);
        let pressed = ui.open_dropdown.is_none() && ui.pressed.as_ref() == Some(&target);
        unsafe {
            let bg = if pressed || is_open {
                palette.control_bg_pressed
            } else if hovered {
                palette.control_bg_hover
            } else {
                palette.control_bg
            };
            let border = if is_open {
                palette.accent_subtle_border
            } else {
                palette.control_border
            };
            draw_elevated_card(rt, rect, CONTROL_RADIUS, bg, border, palette)?;

            let text_brush = rt.CreateSolidColorBrush(&palette.text_primary, None)?;
            let chevron_brush = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;
            let h = rect.bottom - rect.top;
            draw_text(
                rt,
                label,
                &self.formats.body,
                rect_xywh(
                    rect.left + 11.0,
                    rect.top,
                    (rect.right - rect.left) - 32.0,
                    h,
                ),
                &text_brush,
            );
            draw_text(
                rt,
                if is_open {
                    GLYPH_CHEVRON_UP
                } else {
                    GLYPH_CHEVRON_DOWN
                },
                &self.formats.icon_chevron,
                rect_xywh(rect.right - 24.0, rect.top, 16.0, h),
                &chevron_brush,
            );
        }
        self.register_hit(rect, target);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_dropdown_flyout(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        anchor: D2D_RECT_F,
        panel_height: f32,
        options: &[(HitTarget, &str, bool)],
        anim: f32,
        palette: &Palette,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        let menu_w = 132.0_f32.max(anchor.right - anchor.left);
        let item_h = 32.0;
        let pad = 4.0;
        let menu_h = pad * 2.0 + (options.len() as f32) * item_h;
        let slide_y = (1.0 - anim) * -6.0;
        let preferred_top = anchor.bottom + 4.0;
        let max_top = (panel_height - menu_h - 10.0).max(HEADER_HEIGHT + 6.0);
        let menu_top = preferred_top.min(max_top) + slide_y;
        let menu_rect = rect_xywh(anchor.right - menu_w, menu_top, menu_w, menu_h);

        unsafe {
            let s1 =
                rt.CreateSolidColorBrush(&with_alpha(palette.flyout_shadow, anim * 0.45), None)?;
            let s2 =
                rt.CreateSolidColorBrush(&with_alpha(palette.flyout_shadow, anim * 0.85), None)?;
            rt.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: menu_rect.left - 3.0,
                        top: menu_rect.top + 3.0,
                        right: menu_rect.right + 3.0,
                        bottom: menu_rect.bottom + 8.0,
                    },
                    radiusX: CARD_RADIUS + 2.0,
                    radiusY: CARD_RADIUS + 2.0,
                },
                &s1,
            );
            rt.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: menu_rect.left - 1.0,
                        top: menu_rect.top + 1.0,
                        right: menu_rect.right + 1.0,
                        bottom: menu_rect.bottom + 3.0,
                    },
                    radiusX: CARD_RADIUS + 1.0,
                    radiusY: CARD_RADIUS + 1.0,
                },
                &s2,
            );

            let bg = rt.CreateSolidColorBrush(&with_alpha(palette.flyout_bg, anim), None)?;
            let border =
                rt.CreateSolidColorBrush(&with_alpha(palette.flyout_border, anim), None)?;
            let rr = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: menu_rect.left + 0.5,
                    top: menu_rect.top + 0.5,
                    right: menu_rect.right - 0.5,
                    bottom: menu_rect.bottom - 0.5,
                },
                radiusX: CARD_RADIUS,
                radiusY: CARD_RADIUS,
            };
            rt.FillRoundedRectangle(&rr, &bg);
            rt.DrawRoundedRectangle(&rr, &border, 1.0, None);

            let text_primary =
                rt.CreateSolidColorBrush(&with_alpha(palette.text_primary, anim), None)?;
            let accent_brush =
                rt.CreateSolidColorBrush(&with_alpha(palette.accent_fill, anim), None)?;

            for (idx, (target, label, selected)) in options.iter().enumerate() {
                let item_rect = rect_xywh(
                    menu_rect.left + pad,
                    menu_rect.top + pad + (idx as f32) * item_h,
                    menu_w - pad * 2.0,
                    item_h,
                );
                let hovered = ui.hovered.as_ref() == Some(target);
                let pressed = ui.pressed.as_ref() == Some(target);

                if *selected || hovered || pressed {
                    let fill = if pressed {
                        palette.subtle_pressed
                    } else if hovered {
                        palette.card_bg_hover
                    } else {
                        palette.subtle_hover
                    };
                    let hb = rt.CreateSolidColorBrush(&with_alpha(fill, anim), None)?;
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: item_rect,
                            radiusX: CONTROL_RADIUS,
                            radiusY: CONTROL_RADIUS,
                        },
                        &hb,
                    );
                }

                if *selected {
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: rect_xywh(
                                item_rect.left + 1.0,
                                item_rect.top + (item_h - 16.0) * 0.5,
                                3.0,
                                16.0,
                            ),
                            radiusX: 1.5,
                            radiusY: 1.5,
                        },
                        &accent_brush,
                    );
                }

                draw_text(
                    rt,
                    label,
                    &self.formats.body,
                    rect_xywh(
                        item_rect.left + 12.0,
                        item_rect.top,
                        (item_rect.right - item_rect.left) - 18.0,
                        item_h,
                    ),
                    &text_primary,
                );

                self.register_hit(item_rect, target.clone());
            }
        }
        Ok(())
    }

    fn draw_pin_button(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        rect: D2D_RECT_F,
        palette: &Palette,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        let glyph = if ui.pinned { GLYPH_PINNED } else { GLYPH_PIN };
        self.draw_subtle_icon_button(
            rt,
            rect,
            glyph,
            &HitTarget::TogglePin,
            ui.pinned,
            palette,
            ui,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_subtle_icon_button(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        rect: D2D_RECT_F,
        glyph: &str,
        target: &HitTarget,
        active: bool,
        palette: &Palette,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        let hovered = ui.hovered.as_ref() == Some(target);
        let pressed = ui.pressed.as_ref() == Some(target);
        unsafe {
            let rr = D2D1_ROUNDED_RECT {
                rect,
                radiusX: CONTROL_RADIUS,
                radiusY: CONTROL_RADIUS,
            };
            if active {
                let bg = rt.CreateSolidColorBrush(&palette.accent_subtle_bg, None)?;
                let border = rt.CreateSolidColorBrush(&palette.accent_subtle_border, None)?;
                rt.FillRoundedRectangle(&rr, &bg);
                rt.DrawRoundedRectangle(&rr, &border, 1.0, None);
            } else if pressed || hovered {
                let bg = rt.CreateSolidColorBrush(
                    if pressed {
                        &palette.subtle_pressed
                    } else {
                        &palette.subtle_hover
                    },
                    None,
                )?;
                rt.FillRoundedRectangle(&rr, &bg);
            }

            let glyph_color = if active {
                palette.accent_text
            } else if hovered || pressed {
                palette.text_primary
            } else {
                palette.text_secondary
            };
            let glyph_brush = rt.CreateSolidColorBrush(&glyph_color, None)?;
            draw_text(rt, glyph, &self.formats.icon_header, rect, &glyph_brush);
        }
        self.register_hit(rect, target.clone());
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_standard_button(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        rect: D2D_RECT_F,
        glyph: Option<&str>,
        label: &str,
        target: HitTarget,
        palette: &Palette,
        ui: &UiState,
        alpha: f32,
    ) -> windows::core::Result<()> {
        let hovered = ui.hovered.as_ref() == Some(&target);
        let pressed = ui.pressed.as_ref() == Some(&target);
        unsafe {
            let bg_color = if pressed {
                palette.control_bg_pressed
            } else if hovered {
                palette.control_bg_hover
            } else {
                palette.control_bg
            };
            draw_elevated_card(
                rt,
                rect,
                CONTROL_RADIUS,
                with_alpha(bg_color, alpha),
                with_alpha(palette.control_border, alpha),
                palette,
            )?;

            let txt_color = if pressed {
                with_alpha(palette.text_secondary, alpha)
            } else {
                with_alpha(palette.text_primary, alpha)
            };
            let text_brush = rt.CreateSolidColorBrush(&txt_color, None)?;

            if let Some(icon_glyph) = glyph {
                let h = rect.bottom - rect.top;
                let est_text_w = estimate_text_width(label);
                let total_w = 13.0 + 5.0 + est_text_w;
                let start_x = ((rect.left + rect.right - total_w) * 0.5).max(rect.left + 8.0);
                draw_text(
                    rt,
                    icon_glyph,
                    &self.formats.icon_small,
                    rect_xywh(start_x, rect.top, 13.0, h),
                    &text_brush,
                );
                draw_text(
                    rt,
                    label,
                    &self.formats.body,
                    rect_xywh(
                        start_x + 18.0,
                        rect.top,
                        (rect.right - start_x - 22.0).max(20.0),
                        h,
                    ),
                    &text_brush,
                );
            } else {
                draw_text(rt, label, &self.formats.body_center, rect, &text_brush);
            }
        }
        self.register_hit(rect, target);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_toast(
        &self,
        rt: &ID2D1HwndRenderTarget,
        width: f32,
        height: f32,
        palette: &Palette,
        snapshot: &TraySnapshot,
        ui: &UiState,
        msg: &str,
    ) -> windows::core::Result<()> {
        unsafe {
            let est_w = (estimate_text_width(msg) + 52.0).clamp(132.0, width - 32.0);
            let banner_h = 32.0;
            let bottom_offset = if ui.is_compact_pinned(snapshot) {
                12.0
            } else {
                FOOTER_HEIGHT + 12.0
            };
            let rect = D2D_RECT_F {
                left: (width - est_w) * 0.5,
                top: height - bottom_offset - banner_h,
                right: (width + est_w) * 0.5,
                bottom: height - bottom_offset,
            };
            let rr = D2D1_ROUNDED_RECT {
                rect,
                radiusX: CARD_RADIUS,
                radiusY: CARD_RADIUS,
            };
            let b_bg = rt.CreateSolidColorBrush(&palette.toast_bg, None)?;
            let b_border = rt.CreateSolidColorBrush(&palette.toast_border, None)?;
            let b_icon = rt.CreateSolidColorBrush(&palette.status_good, None)?;
            let b_txt = rt.CreateSolidColorBrush(&palette.toast_text, None)?;

            rt.FillRoundedRectangle(&rr, &b_bg);
            rt.DrawRoundedRectangle(&rr, &b_border, 1.0, None);
            draw_text(
                rt,
                GLYPH_CHECKMARK,
                &self.formats.icon_small,
                rect_xywh(rect.left + 12.0, rect.top, 16.0, banner_h),
                &b_icon,
            );
            draw_text(
                rt,
                msg,
                &self.formats.body,
                rect_xywh(rect.left + 32.0, rect.top, est_w - 42.0, banner_h),
                &b_txt,
            );
        }
        Ok(())
    }

    fn register_hit(&mut self, rect: D2D_RECT_F, target: HitTarget) {
        self.hit_regions.push(HitRegion { rect, target });
    }
}

/// Draw one flat independent WinUI 3 `SettingsCard` row (left icon + title/description).
#[allow(clippy::too_many_arguments)]
unsafe fn draw_settings_card_base(
    rt: &ID2D1HwndRenderTarget,
    rect: D2D_RECT_F,
    glyph: &str,
    title: &str,
    desc: &str,
    right_reserved_w: f32,
    formats: &TextFormats,
    icon_brush: &ID2D1SolidColorBrush,
    text_primary: &ID2D1SolidColorBrush,
    text_secondary: &ID2D1SolidColorBrush,
    palette: &Palette,
    hovered: bool,
    pressed: bool,
) -> windows::core::Result<()> {
    let bg = if pressed {
        palette.card_bg_pressed
    } else if hovered {
        palette.card_bg_hover
    } else {
        palette.card_bg
    };
    draw_elevated_card(rt, rect, CARD_RADIUS, bg, palette.card_border, palette)?;
    draw_settings_row_content(
        rt,
        rect,
        glyph,
        title,
        desc,
        right_reserved_w,
        formats,
        icon_brush,
        text_primary,
        text_secondary,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_settings_row_content(
    rt: &ID2D1HwndRenderTarget,
    rect: D2D_RECT_F,
    glyph: &str,
    title: &str,
    desc: &str,
    right_reserved_w: f32,
    formats: &TextFormats,
    icon_brush: &ID2D1SolidColorBrush,
    text_primary: &ID2D1SolidColorBrush,
    text_secondary: &ID2D1SolidColorBrush,
) {
    let card_h = rect.bottom - rect.top;
    draw_text(
        rt,
        glyph,
        &formats.icon_setting,
        rect_xywh(rect.left + 14.0, rect.top, 22.0, card_h),
        icon_brush,
    );

    let text_x = rect.left + 46.0;
    let text_w = ((rect.right - rect.left) - 46.0 - right_reserved_w).max(60.0);
    draw_text(
        rt,
        title,
        &formats.body,
        rect_xywh(text_x, rect.top + 9.0, text_w, 19.0),
        text_primary,
    );
    draw_text(
        rt,
        desc,
        &formats.caption,
        rect_xywh(text_x, rect.top + 29.0, text_w, 18.0),
        text_secondary,
    );
}

/// Draw a WinUI 3 surface card/button with 1px `ControlStrokeColorDefault` border and
/// subtle top (dark mode) or bottom (light mode) elevation bevel highlight.
unsafe fn draw_elevated_card(
    rt: &ID2D1HwndRenderTarget,
    rect: D2D_RECT_F,
    radius: f32,
    fill: D2D1_COLOR_F,
    border: D2D1_COLOR_F,
    palette: &Palette,
) -> windows::core::Result<()> {
    let fill_brush = rt.CreateSolidColorBrush(&fill, None)?;
    let border_brush = rt.CreateSolidColorBrush(&border, None)?;
    let rr = D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F {
            left: rect.left + 0.5,
            top: rect.top + 0.5,
            right: rect.right - 0.5,
            bottom: rect.bottom - 0.5,
        },
        radiusX: radius,
        radiusY: radius,
    };
    rt.FillRoundedRectangle(&rr, &fill_brush);
    rt.DrawRoundedRectangle(&rr, &border_brush, 1.0, None);

    let edge_brush = rt.CreateSolidColorBrush(&palette.elevation_edge, None)?;
    let edge_y = if palette.dark {
        rect.top + 0.5
    } else {
        rect.bottom - 0.5
    };
    rt.DrawLine(
        D2D_POINT_2F {
            x: rect.left + radius,
            y: edge_y,
        },
        D2D_POINT_2F {
            x: rect.right - radius,
            y: edge_y,
        },
        &edge_brush,
        1.0,
        None,
    );
    Ok(())
}

/// Draw an authentic animated Windows 11 Fluent 2 `ToggleSwitch` (40x20 DIPs).
unsafe fn draw_win11_toggle_switch(
    rt: &ID2D1HwndRenderTarget,
    x: f32,
    y: f32,
    t: f32,
    hovered: bool,
    pressed: bool,
    palette: &Palette,
) -> windows::core::Result<()> {
    let t = t.clamp(0.0, 1.0);
    let sw_rect = rect_xywh(x + 0.5, y + 0.5, 39.0, 19.0);
    let rr = D2D1_ROUNDED_RECT {
        rect: sw_rect,
        radiusX: 9.5,
        radiusY: 9.5,
    };

    let off_bg = if hovered {
        palette.toggle_off_bg_hover
    } else {
        palette.toggle_off_bg
    };
    let on_bg = if hovered {
        palette.accent_fill_hover
    } else {
        palette.accent_fill
    };
    let track_color = lerp_color(off_bg, on_bg, t);
    let track_brush = rt.CreateSolidColorBrush(&track_color, None)?;
    rt.FillRoundedRectangle(&rr, &track_brush);

    if t < 0.99 {
        let border_color = with_alpha(palette.toggle_off_border, 1.0 - t);
        let border_brush = rt.CreateSolidColorBrush(&border_color, None)?;
        rt.DrawRoundedRectangle(&rr, &border_brush, 1.0, None);
    }

    let thumb_h = if pressed || hovered { 14.0 } else { 12.0 };
    let thumb_w = if pressed { 17.0 } else { thumb_h };
    let left_x = x + 4.0;
    let right_x = x + 36.0 - thumb_w;
    let thumb_x = left_x + (right_x - left_x) * t;
    let thumb_y = y + (20.0 - thumb_h) * 0.5;

    let thumb_color = lerp_color(palette.toggle_off_thumb, palette.text_on_accent, t);
    let thumb_brush = rt.CreateSolidColorBrush(&thumb_color, None)?;
    rt.FillRoundedRectangle(
        &D2D1_ROUNDED_RECT {
            rect: rect_xywh(thumb_x, thumb_y, thumb_w, thumb_h),
            radiusX: thumb_h * 0.5,
            radiusY: thumb_h * 0.5,
        },
        &thumb_brush,
    );
    Ok(())
}

fn estimate_text_width(text: &str) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { 7.0 } else { 12.5 })
        .sum()
}

fn device_kind_glyph(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Mouse => GLYPH_MOUSE,
        DeviceKind::Keyboard => GLYPH_KEYBOARD,
        DeviceKind::Gamepad => GLYPH_GAMEPAD,
        DeviceKind::Headset => GLYPH_HEADSET,
        DeviceKind::Peripheral => GLYPH_PERIPHERAL,
    }
}

fn resolve_font_family(dwrite: &IDWriteFactory, preferred: PCWSTR, fallback: PCWSTR) -> PCWSTR {
    unsafe {
        let mut collection = None;
        if dwrite
            .GetSystemFontCollection(&mut collection, false)
            .is_ok()
        {
            if let Some(collection) = collection {
                let mut index = 0u32;
                let mut exists = BOOL(0);
                if collection
                    .FindFamilyName(preferred, &mut index, &mut exists)
                    .is_ok()
                    && exists.as_bool()
                {
                    return preferred;
                }
            }
        }
    }
    fallback
}

#[allow(clippy::too_many_arguments)]
fn create_format(
    dwrite: &IDWriteFactory,
    font_family: PCWSTR,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    size: f32,
    align: windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT,
    para_align: windows::Win32::Graphics::DirectWrite::DWRITE_PARAGRAPH_ALIGNMENT,
    wrap: bool,
    ellipsis: bool,
) -> windows::core::Result<IDWriteTextFormat> {
    unsafe {
        let fmt = dwrite.CreateTextFormat(
            font_family,
            None,
            weight,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            w!("zh-CN"),
        )?;
        fmt.SetTextAlignment(align)?;
        fmt.SetParagraphAlignment(para_align)?;
        fmt.SetWordWrapping(if wrap {
            DWRITE_WORD_WRAPPING_WRAP
        } else {
            DWRITE_WORD_WRAPPING_NO_WRAP
        })?;
        if ellipsis && !wrap {
            if let Ok(trimming_sign) = dwrite.CreateEllipsisTrimmingSign(&fmt) {
                let trimming = DWRITE_TRIMMING {
                    granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    delimiter: 0,
                    delimiterCount: 0,
                };
                let _ = fmt.SetTrimming(&trimming, &trimming_sign);
            }
        }
        Ok(fmt)
    }
}

fn draw_text(
    rt: &ID2D1HwndRenderTarget,
    text: &str,
    format: &IDWriteTextFormat,
    rect: D2D_RECT_F,
    brush: &ID2D1SolidColorBrush,
) {
    let wide = HSTRING::from(text);
    unsafe {
        rt.DrawText(
            wide.as_wide(),
            format,
            &rect,
            brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
}

fn draw_text_shadowed(
    rt: &ID2D1HwndRenderTarget,
    text: &str,
    format: &IDWriteTextFormat,
    rect: D2D_RECT_F,
    brush: &ID2D1SolidColorBrush,
    shadow: Option<&ID2D1SolidColorBrush>,
) {
    if let Some(sb) = shadow {
        let s_rect = D2D_RECT_F {
            left: rect.left,
            top: rect.top + 1.0,
            right: rect.right,
            bottom: rect.bottom + 1.0,
        };
        draw_text(rt, text, format, s_rect, sb);
    }
    draw_text(rt, text, format, rect, brush);
}

fn rect_xywh(x: f32, y: f32, w: f32, h: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}

fn battery_tone(dev: &DeviceView, palette: &Palette) -> (D2D1_COLOR_F, D2D1_COLOR_F) {
    let is_stale = dev.freshness == Some(Freshness::Stale);
    if dev.charge_state == ChargeState::Charging {
        return (palette.status_good, palette.status_good);
    }
    match dev.precision {
        Precision::Percentage => match dev.percentage {
            Some(pct) if pct <= 15 => (palette.status_critical, palette.status_critical),
            Some(pct) if pct <= 30 => (palette.status_caution, palette.status_caution),
            Some(_) if is_stale => (palette.text_secondary, palette.status_good_muted),
            Some(_) => (palette.text_primary, palette.status_good),
            None => (palette.text_subtle, palette.bar_track),
        },
        Precision::DiscreteLevel => match dev.level {
            Some(BatteryLevel::Critical | BatteryLevel::Low) => {
                (palette.status_caution, palette.status_caution)
            }
            Some(_) => (palette.text_primary, palette.status_good),
            None => (palette.text_subtle, palette.bar_track),
        },
        Precision::VoltageOnly => (palette.text_primary, palette.accent_fill),
        Precision::None => (palette.text_subtle, palette.bar_track),
    }
}
