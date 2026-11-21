//! Direct2D + DirectWrite renderer and hit-testing engine for the Win11 Fluent popover.
//!
//! All UI icons are rendered from Windows' built-in `Segoe Fluent Icons` font
//! (falling back to `Segoe MDL2 Assets` on Windows 10) for crisp, native optical
//! fidelity across all DPI scales.

use crate::icon::resolve_icon_font_family;
use crate::theme::{rgba, Palette};
use chrono::{DateTime, Utc};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F,
    D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
    D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_ROUNDED_RECT, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_MEDIUM,
    DWRITE_FONT_WEIGHT_REGULAR, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_WORD_WRAPPING_NO_WRAP, DWRITE_WORD_WRAPPING_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use wireless_battery::{
    BatteryLevel, ChargeState, ErrorKind, Freshness, Precision, Reachability, SupportStatus,
};
use wireless_battery_tray_core::{DeviceKind, DeviceView, TrayIconState, TraySnapshot};

/// Logical popover width in DIPs (matches our HTML design prototype).
pub const POPOVER_WIDTH_DIP: f32 = 344.0;
pub const POPOVER_MAX_HEIGHT_DIP: f32 = 476.0;
pub const HEADER_HEIGHT: f32 = 46.0;

const FOOTER_HEIGHT: f32 = 38.0;
const LIST_PADDING: f32 = 10.0;
const CARD_GAP: f32 = 6.0;
const CARD_COLLAPSED_H: f32 = 60.0;
const CARD_EXPANDED_EXTRA_H: f32 = 88.0;

// Built-in Windows `Segoe Fluent Icons` / `Segoe MDL2 Assets` codepoints.
const GLYPH_PIN: &str = "\u{E718}";
const GLYPH_PINNED: &str = "\u{E840}";
const GLYPH_REFRESH: &str = "\u{E72C}";
const GLYPH_SETTINGS: &str = "\u{E713}";
const GLYPH_CHEVRON_LEFT: &str = "\u{E76B}";
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

/// Interactive targets registered during every paint pass for pixel-accurate hit-testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitTarget {
    TogglePin,
    RefreshAll,
    OpenSettings,
    BackToMain,
    Quit,
    ToggleDevice(String),
    RefreshDevice(String),
    CopyDiagnostic(String),
    OpenSettingsFromEmpty,
    SetPollInterval(u64),
    ToggleLowBatteryNotify,
    ToggleAutostart,
}

#[derive(Debug, Clone)]
struct HitRegion {
    rect: D2D_RECT_F,
    target: HitTarget,
}

/// Ephemeral UI state owned by the popover window.
pub struct UiState {
    pub view: PopoverView,
    pub pinned: bool,
    pub mouse_inside: bool,
    pub expanded_device_id: Option<String>,
    pub scroll_offset: f32,
    pub hovered: Option<HitTarget>,
    pub feedback_banner: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            view: PopoverView::Main,
            pinned: false,
            mouse_inside: false,
            expanded_device_id: None,
            scroll_offset: 0.0,
            hovered: None,
            feedback_banner: None,
        }
    }
}

impl UiState {
    /// True when pinned in compact floating mode (cursor outside): hides header and footer,
    /// leaving only the middle device cards visible.
    pub fn is_compact_pinned(&self, snapshot: &TraySnapshot) -> bool {
        self.pinned
            && !self.mouse_inside
            && self.view == PopoverView::Main
            && !snapshot.devices.is_empty()
    }
}

/// Target surface parameters passed to `PopoverRenderer::paint`.
#[derive(Debug, Clone, Copy)]
pub struct PaintSurface {
    pub hwnd: HWND,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub dpi: u32,
    pub dark: bool,
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
    icon_small: IDWriteTextFormat,
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

        let formats = TextFormats {
            title: create_format(
                &dwrite,
                w!("Segoe UI Variable Text"),
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                13.0,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
            )?,
            device_name: create_format(
                &dwrite,
                w!("Segoe UI Variable Text"),
                DWRITE_FONT_WEIGHT_MEDIUM,
                13.0,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
            )?,
            value_large: create_format(
                &dwrite,
                w!("Segoe UI Variable Display"),
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                15.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
            )?,
            value_discrete: create_format(
                &dwrite,
                w!("Segoe UI Variable Text"),
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                13.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
            )?,
            body: create_format(
                &dwrite,
                w!("Segoe UI Variable Text"),
                DWRITE_FONT_WEIGHT_REGULAR,
                12.0,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
            )?,
            body_center: create_format(
                &dwrite,
                w!("Segoe UI Variable Text"),
                DWRITE_FONT_WEIGHT_MEDIUM,
                11.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
            )?,
            caption: create_format(
                &dwrite,
                w!("Segoe UI Variable Small"),
                DWRITE_FONT_WEIGHT_REGULAR,
                11.0,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
            )?,
            caption_right: create_format(
                &dwrite,
                w!("Segoe UI Variable Small"),
                DWRITE_FONT_WEIGHT_REGULAR,
                11.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
            )?,
            caption_center: create_format(
                &dwrite,
                w!("Segoe UI Variable Small"),
                DWRITE_FONT_WEIGHT_REGULAR,
                11.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                true,
            )?,
            mono_right: create_format(
                &dwrite,
                w!("Cascadia Mono"),
                DWRITE_FONT_WEIGHT_REGULAR,
                10.5,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
            )?,
            icon_header: create_format(
                &dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                13.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
            )?,
            icon_device: create_format(
                &dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                16.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
            )?,
            icon_small: create_format(
                &dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
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
            PopoverView::Settings => 256.0,
            PopoverView::Main => {
                if snapshot.devices.is_empty() {
                    return HEADER_HEIGHT + 178.0 + FOOTER_HEIGHT;
                }
                let mut list_h = LIST_PADDING * 2.0;
                for (idx, dev) in snapshot.devices.iter().enumerate() {
                    if idx > 0 {
                        list_h += CARD_GAP;
                    }
                    list_h += CARD_COLLAPSED_H;
                    if ui.expanded_device_id.as_deref() == Some(dev.id.as_str()) {
                        list_h += CARD_EXPANDED_EXTRA_H;
                    }
                }
                if ui.is_compact_pinned(snapshot) {
                    list_h.clamp(80.0, POPOVER_MAX_HEIGHT_DIP - HEADER_HEIGHT - FOOTER_HEIGHT)
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
        // Iterate in reverse so child buttons inside cards win over the enclosing card rect.
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
        let palette = Palette::for_mode(surface.dark);

        self.hit_regions.clear();

        unsafe {
            rt.BeginDraw();
            // Clear to full transparency so Win11 DWM Transient Acrylic composites cleanly behind us.
            rt.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            let bg_color = if surface.acrylic_active {
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

            match ui.view {
                PopoverView::Main => {
                    self.draw_main_view(&rt, width, height, &palette, snapshot, ui)?;
                }
                PopoverView::Settings => {
                    self.draw_settings_view(&rt, width, height, &palette, snapshot, ui)?;
                }
            }

            if let Some(msg) = &ui.feedback_banner {
                let banner_w = 210.0;
                let banner_h = 26.0;
                let bottom_offset = if ui.is_compact_pinned(snapshot) {
                    8.0
                } else {
                    FOOTER_HEIGHT + 8.0
                };
                let rect = D2D_RECT_F {
                    left: (width - banner_w) * 0.5,
                    top: height - bottom_offset - banner_h,
                    right: (width + banner_w) * 0.5,
                    bottom: height - bottom_offset,
                };
                let b_bg = rt.CreateSolidColorBrush(&palette.toast_bg, None)?;
                let b_txt = rt.CreateSolidColorBrush(&palette.toast_text, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect,
                        radiusX: 13.0,
                        radiusY: 13.0,
                    },
                    &b_bg,
                );
                draw_text(&rt, msg, &self.formats.body_center, rect, &b_txt);
            }

            // Crisp 1px inner border.
            let border_brush = rt.CreateSolidColorBrush(&palette.outer_border, None)?;
            rt.DrawRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: 0.5,
                        top: 0.5,
                        right: width - 0.5,
                        bottom: height - 0.5,
                    },
                    radiusX: 8.0,
                    radiusY: 8.0,
                },
                &border_brush,
                1.0,
                None,
            );

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
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };
        let target = unsafe { self.d2d.CreateHwndRenderTarget(&props, &hwnd_props)? };
        unsafe {
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        }
        self.target = Some(target.clone());
        Ok(target)
    }

    fn draw_main_view(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        width: f32,
        height: f32,
        palette: &Palette,
        snapshot: &TraySnapshot,
        ui: &mut UiState,
    ) -> windows::core::Result<()> {
        let compact = ui.is_compact_pinned(snapshot);
        unsafe {
            let text_primary = rt.CreateSolidColorBrush(&palette.text_primary, None)?;
            let text_secondary = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;
            let text_subtle = rt.CreateSolidColorBrush(&palette.text_subtle, None)?;
            let divider_brush = rt.CreateSolidColorBrush(&palette.divider, None)?;

            if !compact {
                // --- Header ---
                let dot_color = match snapshot.icon_state {
                    TrayIconState::Idle => palette.text_subtle,
                    TrayIconState::LowBattery => palette.accent_rose,
                    TrayIconState::Normal => {
                        let all_stale = !snapshot.devices.is_empty()
                            && snapshot
                                .devices
                                .iter()
                                .all(|d| d.freshness == Some(Freshness::Stale));
                        if all_stale {
                            palette.accent_amber
                        } else {
                            palette.accent_green
                        }
                    }
                };
                let dot_brush = rt.CreateSolidColorBrush(&dot_color, None)?;
                rt.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: D2D_POINT_2F { x: 20.0, y: 23.0 },
                        radiusX: 3.5,
                        radiusY: 3.5,
                    },
                    &dot_brush,
                );

                draw_text(
                    rt,
                    "Wireless Battery",
                    &self.formats.title,
                    rect_xywh(30.0, 10.0, 114.0, 26.0),
                    &text_primary,
                );

                let sync_text = if snapshot.refreshing {
                    "· 读取中…".to_owned()
                } else {
                    format!("· {}", format_relative_time(snapshot.last_sync_at))
                };
                draw_text(
                    rt,
                    &sync_text,
                    &self.formats.caption,
                    rect_xywh(142.0, 10.0, 92.0, 26.0),
                    &text_subtle,
                );

                // Header action buttons: native Fluent Pin + Refresh + Settings glyphs
                let pin_rect = rect_xywh(width - 104.0, 9.0, 28.0, 28.0);
                let refresh_rect = rect_xywh(width - 72.0, 9.0, 28.0, 28.0);
                let settings_rect = rect_xywh(width - 40.0, 9.0, 28.0, 28.0);
                self.draw_pin_button(rt, pin_rect, palette, ui)?;
                self.draw_glyph_button(
                    rt,
                    refresh_rect,
                    GLYPH_REFRESH,
                    &HitTarget::RefreshAll,
                    palette,
                    ui,
                )?;
                self.draw_glyph_button(
                    rt,
                    settings_rect,
                    GLYPH_SETTINGS,
                    &HitTarget::OpenSettings,
                    palette,
                    ui,
                )?;

                rt.DrawLine(
                    D2D_POINT_2F {
                        x: 0.0,
                        y: HEADER_HEIGHT,
                    },
                    D2D_POINT_2F {
                        x: width,
                        y: HEADER_HEIGHT,
                    },
                    &divider_brush,
                    1.0,
                    None,
                );
            }

            // --- Scrollable Device List Area ---
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
                windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
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
                    snapshot,
                    ui,
                )?;
            } else {
                let mut total_h = LIST_PADDING * 2.0;
                for (idx, dev) in snapshot.devices.iter().enumerate() {
                    if idx > 0 {
                        total_h += CARD_GAP;
                    }
                    total_h += CARD_COLLAPSED_H;
                    if ui.expanded_device_id.as_deref() == Some(dev.id.as_str()) {
                        total_h += CARD_EXPANDED_EXTRA_H;
                    }
                }
                self.content_height = total_h;
                ui.scroll_offset = ui.scroll_offset.clamp(0.0, self.max_scroll());

                let mut y = list_top + LIST_PADDING - ui.scroll_offset;
                for dev in &snapshot.devices {
                    let expanded = ui.expanded_device_id.as_deref() == Some(dev.id.as_str());
                    let card_h = if expanded {
                        CARD_COLLAPSED_H + CARD_EXPANDED_EXTRA_H
                    } else {
                        CARD_COLLAPSED_H
                    };
                    let card_rect = rect_xywh(10.0, y, width - 20.0, card_h);
                    if card_rect.bottom >= list_top - 4.0 && card_rect.top <= list_bottom + 4.0 {
                        self.draw_device_card(rt, card_rect, dev, expanded, palette, ui)?;
                    }
                    y += card_h + CARD_GAP;
                }
            }

            rt.PopAxisAlignedClip();

            if !compact {
                // --- Footer ---
                rt.DrawLine(
                    D2D_POINT_2F {
                        x: 0.0,
                        y: list_bottom,
                    },
                    D2D_POINT_2F {
                        x: width,
                        y: list_bottom,
                    },
                    &divider_brush,
                    1.0,
                    None,
                );

                let policy_label = match snapshot.config.poll_interval_secs {
                    0 => "仅手动刷新".to_owned(),
                    secs if secs % 60 == 0 => format!("每 {} 分钟静默轮询", secs / 60),
                    secs => format!("每 {secs} 秒静默轮询"),
                };
                draw_text(
                    rt,
                    &policy_label,
                    &self.formats.caption,
                    rect_xywh(14.0, list_bottom, 210.0, FOOTER_HEIGHT),
                    &text_secondary,
                );

                let quit_rect =
                    rect_xywh(width - 54.0, list_bottom + 6.0, 42.0, FOOTER_HEIGHT - 12.0);
                let quit_hovered = ui.hovered == Some(HitTarget::Quit);
                if quit_hovered {
                    let hover_b = rt.CreateSolidColorBrush(&palette.pill_bg, None)?;
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: quit_rect,
                            radiusX: 6.0,
                            radiusY: 6.0,
                        },
                        &hover_b,
                    );
                }
                draw_text(
                    rt,
                    "退出",
                    &self.formats.body_center,
                    quit_rect,
                    if quit_hovered {
                        &text_primary
                    } else {
                        &text_subtle
                    },
                );
                self.register_hit(quit_rect, HitTarget::Quit);
            }
        }
        Ok(())
    }

    fn draw_empty_state(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        bounds: D2D_RECT_F,
        palette: &Palette,
        snapshot: &TraySnapshot,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        let width = bounds.right - bounds.left;
        let top = bounds.top;
        let bottom = bounds.bottom;
        unsafe {
            let center_y = top + (bottom - top) * 0.5;
            let well_rect = rect_xywh((width - 40.0) * 0.5, center_y - 62.0, 40.0, 40.0);
            let well_brush = rt.CreateSolidColorBrush(&palette.icon_well, None)?;
            let glyph_brush = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;
            let text_primary = rt.CreateSolidColorBrush(&palette.text_primary, None)?;
            let text_muted = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;

            rt.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: well_rect,
                    radiusX: 11.0,
                    radiusY: 11.0,
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
                "未发现可读取的无线外设",
                &self.formats.body_center,
                rect_xywh(20.0, center_y - 14.0, width - 40.0, 22.0),
                &text_primary,
            );

            let hint = if snapshot.hidden_experimental_count > 0 {
                format!(
                    "已检测到 {} 台实验性协议设备，默认未启用。可在设置中开启。",
                    snapshot.hidden_experimental_count
                )
            } else {
                "被动枚举未匹配到已知接收器或外设，请确认接收器已接入并处于唤醒状态。".to_owned()
            };
            draw_text(
                rt,
                &hint,
                &self.formats.caption_center,
                rect_xywh(34.0, center_y + 10.0, width - 68.0, 36.0),
                &text_muted,
            );

            let btn_w = if snapshot.hidden_experimental_count > 0 {
                136.0
            } else {
                104.0
            };
            let btn_rect = rect_xywh((width - btn_w) * 0.5, center_y + 52.0, btn_w, 26.0);
            let target = if snapshot.hidden_experimental_count > 0 {
                HitTarget::OpenSettingsFromEmpty
            } else {
                HitTarget::RefreshAll
            };
            let hovered = ui.hovered.as_ref() == Some(&target);
            let btn_bg = rt.CreateSolidColorBrush(
                if hovered {
                    &palette.pill_bg_hover
                } else {
                    &palette.pill_bg
                },
                None,
            )?;
            rt.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: btn_rect,
                    radiusX: 7.0,
                    radiusY: 7.0,
                },
                &btn_bg,
            );
            draw_text(
                rt,
                if snapshot.hidden_experimental_count > 0 {
                    "前往开启实验性协议"
                } else {
                    "重新扫描端点"
                },
                &self.formats.body_center,
                btn_rect,
                &text_primary,
            );
            self.register_hit(btn_rect, target);
        }
        Ok(())
    }

    fn draw_device_card(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        rect: D2D_RECT_F,
        dev: &DeviceView,
        expanded: bool,
        palette: &Palette,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        unsafe {
            let toggle_target = HitTarget::ToggleDevice(dev.id.clone());
            let hovered = ui.hovered.as_ref() == Some(&toggle_target);

            let bg_color = if expanded {
                palette.card_bg_expanded
            } else if hovered {
                palette.card_bg_hover
            } else {
                palette.card_bg
            };
            let border_color = if hovered || expanded {
                palette.card_border_hover
            } else {
                palette.card_border
            };

            let bg_brush = rt.CreateSolidColorBrush(&bg_color, None)?;
            let border_brush = rt.CreateSolidColorBrush(&border_color, None)?;
            let rr = D2D1_ROUNDED_RECT {
                rect,
                radiusX: 10.0,
                radiusY: 10.0,
            };
            rt.FillRoundedRectangle(&rr, &bg_brush);
            rt.DrawRoundedRectangle(&rr, &border_brush, 1.0, None);

            self.register_hit(rect, toggle_target);

            // Native Fluent hardware icon inside subtle rounded well
            let well_rect = rect_xywh(rect.left + 11.0, rect.top + 12.0, 36.0, 36.0);
            let well_brush = rt.CreateSolidColorBrush(&palette.icon_well, None)?;
            let glyph_brush = rt.CreateSolidColorBrush(&palette.icon_stroke, None)?;
            rt.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: well_rect,
                    radiusX: 9.0,
                    radiusY: 9.0,
                },
                &well_brush,
            );
            draw_text(
                rt,
                device_kind_glyph(dev.kind),
                &self.formats.icon_device,
                well_rect,
                &glyph_brush,
            );

            // Name & clean 1-line subtitle (no engineering badges in collapsed state!)
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
                palette.accent_green
            } else if dev.precision == Precision::None && dev.last_error.is_some() {
                palette.accent_amber
            } else {
                palette.text_secondary
            };
            let sub_brush = rt.CreateSolidColorBrush(&sub_color, None)?;

            let text_left = rect.left + 57.0;
            let text_width = (rect.right - rect.left) - 146.0;
            draw_text(
                rt,
                &dev.name,
                &self.formats.device_name,
                rect_xywh(text_left, rect.top + 12.0, text_width, 19.0),
                &name_brush,
            );

            let subtitle = format_device_subtitle(dev);
            draw_text(
                rt,
                &subtitle,
                &self.formats.caption,
                rect_xywh(text_left, rect.top + 31.0, text_width, 18.0),
                &sub_brush,
            );

            // Right-side honest telemetry readout
            let right_w = 72.0;
            let right_x = rect.right - right_w - 12.0;
            let bar_y = rect.top + 38.0;
            let track_brush = rt.CreateSolidColorBrush(&palette.bar_track, None)?;

            let (val_color, bar_color) = battery_tone(dev, palette);
            let val_brush = rt.CreateSolidColorBrush(&val_color, None)?;
            let fill_brush = rt.CreateSolidColorBrush(&bar_color, None)?;

            match dev.precision {
                Precision::Percentage => {
                    if let Some(pct) = dev.percentage {
                        if dev.charge_state == ChargeState::Charging {
                            let bolt_brush =
                                rt.CreateSolidColorBrush(&palette.accent_green, None)?;
                            draw_text(
                                rt,
                                GLYPH_CHARGING_BOLT,
                                &self.formats.icon_small,
                                rect_xywh(right_x + 8.0, rect.top + 15.0, 14.0, 16.0),
                                &bolt_brush,
                            );
                        }
                        let label = format!("{pct}%");
                        draw_text(
                            rt,
                            &label,
                            &self.formats.value_large,
                            rect_xywh(right_x, rect.top + 13.0, right_w, 20.0),
                            &val_brush,
                        );
                        let track_rect = rect_xywh(right_x, bar_y, right_w, 3.5);
                        rt.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: track_rect,
                                radiusX: 1.75,
                                radiusY: 1.75,
                            },
                            &track_brush,
                        );
                        let fill_w = (right_w * (pct as f32 / 100.0)).clamp(3.0, right_w);
                        rt.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: rect_xywh(right_x, bar_y, fill_w, 3.5),
                                radiusX: 1.75,
                                radiusY: 1.75,
                            },
                            &fill_brush,
                        );
                    }
                }
                Precision::DiscreteLevel => {
                    let (segments, label) = discrete_info(dev.level);
                    draw_text(
                        rt,
                        label,
                        &self.formats.value_discrete,
                        rect_xywh(right_x, rect.top + 14.0, right_w, 19.0),
                        &val_brush,
                    );
                    let gap = 3.0;
                    let seg_w = (right_w - gap * 3.0) / 4.0;
                    for i in 0..4u8 {
                        let sx = right_x + (i as f32) * (seg_w + gap);
                        let seg_rect = rect_xywh(sx, bar_y, seg_w, 3.5);
                        let b = if i < segments {
                            &fill_brush
                        } else {
                            &track_brush
                        };
                        rt.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: seg_rect,
                                radiusX: 1.75,
                                radiusY: 1.75,
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
                    draw_text(
                        rt,
                        &label,
                        &self.formats.value_discrete,
                        rect_xywh(right_x, rect.top + 14.0, right_w, 19.0),
                        &val_brush,
                    );
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: rect_xywh(right_x, bar_y, right_w, 3.5),
                            radiusX: 1.75,
                            radiusY: 1.75,
                        },
                        &track_brush,
                    );
                }
                Precision::None => {
                    let subtle_brush = rt.CreateSolidColorBrush(&palette.text_subtle, None)?;
                    draw_text(
                        rt,
                        "--",
                        &self.formats.value_large,
                        rect_xywh(right_x, rect.top + 13.0, right_w, 20.0),
                        &subtle_brush,
                    );
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: rect_xywh(right_x, bar_y, right_w, 3.5),
                            radiusX: 1.75,
                            radiusY: 1.75,
                        },
                        &track_brush,
                    );
                }
            }

            // Expanded diagnostics & evidence drawer (in-place)
            if expanded {
                let div_y = rect.top + CARD_COLLAPSED_H;
                let divider_brush = rt.CreateSolidColorBrush(&palette.divider, None)?;
                let subtle_brush = rt.CreateSolidColorBrush(&palette.text_subtle, None)?;
                let sec_brush = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;

                rt.DrawLine(
                    D2D_POINT_2F {
                        x: rect.left + 12.0,
                        y: div_y,
                    },
                    D2D_POINT_2F {
                        x: rect.right - 12.0,
                        y: div_y,
                    },
                    &divider_brush,
                    1.0,
                    None,
                );

                let inner_w = (rect.right - rect.left) - 24.0;
                let col_w = (inner_w - 14.0) * 0.5;
                let col1_x = rect.left + 12.0;
                let col2_x = col1_x + col_w + 14.0;
                let row1_y = div_y + 8.0;
                let row2_y = row1_y + 20.0;

                draw_text(
                    rt,
                    "协议",
                    &self.formats.caption,
                    rect_xywh(col1_x, row1_y, 34.0, 18.0),
                    &subtle_brush,
                );
                draw_text(
                    rt,
                    dev.provider_id.as_deref().unwrap_or("--"),
                    &self.formats.mono_right,
                    rect_xywh(col1_x + 30.0, row1_y, col_w - 30.0, 18.0),
                    &sec_brush,
                );

                draw_text(
                    rt,
                    "证据",
                    &self.formats.caption,
                    rect_xywh(col2_x, row1_y, 34.0, 18.0),
                    &subtle_brush,
                );
                let (ev_text, ev_color) = match dev.support_status {
                    SupportStatus::Verified => ("Verified", palette.accent_green),
                    SupportStatus::Experimental => ("Supported", palette.accent_green),
                    SupportStatus::Deprecated => ("Deprecated", palette.text_secondary),
                    _ => ("Unknown", palette.text_subtle),
                };
                let ev_brush = rt.CreateSolidColorBrush(&ev_color, None)?;
                draw_text(
                    rt,
                    ev_text,
                    &self.formats.caption_right,
                    rect_xywh(col2_x + 30.0, row1_y, col_w - 30.0, 18.0),
                    &ev_brush,
                );

                draw_text(
                    rt,
                    "总线",
                    &self.formats.caption,
                    rect_xywh(col1_x, row2_y, 34.0, 18.0),
                    &subtle_brush,
                );
                draw_text(
                    rt,
                    &dev.transport_label,
                    &self.formats.caption_right,
                    rect_xywh(col1_x + 30.0, row2_y, col_w - 30.0, 18.0),
                    &sec_brush,
                );

                draw_text(
                    rt,
                    "状态",
                    &self.formats.caption,
                    rect_xywh(col2_x, row2_y, 34.0, 18.0),
                    &subtle_brush,
                );
                let status_text = if is_stale {
                    "Stale（缓存）"
                } else if dev.freshness == Some(Freshness::Live) {
                    "Live（实时）"
                } else if let Some(err) = dev.last_error {
                    error_kind_short_label(err)
                } else {
                    "未读取"
                };
                draw_text(
                    rt,
                    status_text,
                    &self.formats.caption_right,
                    rect_xywh(col2_x + 32.0, row2_y, col_w - 32.0, 18.0),
                    &sec_brush,
                );

                let btn_y = row2_y + 26.0;
                let diag_rect = rect_xywh(rect.right - 104.0, btn_y, 92.0, 24.0);
                let read_rect = rect_xywh(diag_rect.left - 70.0, btn_y, 64.0, 24.0);

                let read_target = HitTarget::RefreshDevice(dev.id.clone());
                let diag_target = HitTarget::CopyDiagnostic(dev.id.clone());

                self.draw_pill_button(rt, read_rect, "重新读取", read_target, palette, ui)?;
                self.draw_pill_button(rt, diag_rect, "复制脱敏诊断", diag_target, palette, ui)?;
            }
        }
        Ok(())
    }

    fn draw_settings_view(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        width: f32,
        height: f32,
        palette: &Palette,
        snapshot: &TraySnapshot,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        unsafe {
            let text_primary = rt.CreateSolidColorBrush(&palette.text_primary, None)?;
            let text_secondary = rt.CreateSolidColorBrush(&palette.text_secondary, None)?;
            let text_subtle = rt.CreateSolidColorBrush(&palette.text_subtle, None)?;
            let divider_brush = rt.CreateSolidColorBrush(&palette.divider, None)?;

            let back_rect = rect_xywh(10.0, 10.0, 62.0, 26.0);
            let back_hovered = ui.hovered == Some(HitTarget::BackToMain);
            if back_hovered {
                let hb = rt.CreateSolidColorBrush(&palette.pill_bg, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: back_rect,
                        radiusX: 7.0,
                        radiusY: 7.0,
                    },
                    &hb,
                );
            }
            draw_text(
                rt,
                GLYPH_CHEVRON_LEFT,
                &self.formats.icon_small,
                rect_xywh(14.0, 11.0, 14.0, 24.0),
                &text_primary,
            );
            draw_text(
                rt,
                "返回",
                &self.formats.body,
                rect_xywh(30.0, 10.0, 38.0, 26.0),
                &text_primary,
            );
            self.register_hit(back_rect, HitTarget::BackToMain);

            draw_text(
                rt,
                "偏好与安全策略",
                &self.formats.body_center,
                rect_xywh(80.0, 10.0, width - 160.0, 26.0),
                &text_primary,
            );

            rt.DrawLine(
                D2D_POINT_2F {
                    x: 0.0,
                    y: HEADER_HEIGHT,
                },
                D2D_POINT_2F {
                    x: width,
                    y: HEADER_HEIGHT,
                },
                &divider_brush,
                1.0,
                None,
            );

            let pad_x = 16.0;
            let inner_w = width - pad_x * 2.0;
            let mut y = HEADER_HEIGHT + 14.0;

            draw_text(
                rt,
                "轮询频率",
                &self.formats.body,
                rect_xywh(pad_x, y, 100.0, 18.0),
                &text_primary,
            );
            draw_text(
                rt,
                "休眠设备不主动唤醒",
                &self.formats.caption_right,
                rect_xywh(pad_x + 110.0, y, inner_w - 110.0, 18.0),
                &text_subtle,
            );
            y += 24.0;

            let seg_outer = rect_xywh(pad_x, y, inner_w, 30.0);
            let pill_bg = rt.CreateSolidColorBrush(&palette.pill_bg, None)?;
            let pill_active = rt.CreateSolidColorBrush(&palette.pill_active, None)?;
            rt.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: seg_outer,
                    radiusX: 8.0,
                    radiusY: 8.0,
                },
                &pill_bg,
            );

            let options: [(u64, &str); 4] = [
                (300, "5 分钟"),
                (900, "15 分钟"),
                (1800, "30 分钟"),
                (0, "手动"),
            ];
            let seg_w = (inner_w - 6.0) / 4.0;
            for (idx, (secs, label)) in options.iter().enumerate() {
                let r = rect_xywh(pad_x + 3.0 + (idx as f32) * seg_w, y + 3.0, seg_w, 24.0);
                let active = snapshot.config.poll_interval_secs == *secs;
                if active {
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: r,
                            radiusX: 6.0,
                            radiusY: 6.0,
                        },
                        &pill_active,
                    );
                }
                draw_text(
                    rt,
                    label,
                    &self.formats.body_center,
                    r,
                    if active {
                        &text_primary
                    } else {
                        &text_secondary
                    },
                );
                self.register_hit(r, HitTarget::SetPollInterval(*secs));
            }

            y += 44.0;
            rt.DrawLine(
                D2D_POINT_2F { x: pad_x, y },
                D2D_POINT_2F {
                    x: width - pad_x,
                    y,
                },
                &divider_brush,
                1.0,
                None,
            );
            y += 12.0;

            let cfg = &snapshot.config;
            let toggles = [
                (
                    "低电量系统提醒（≤ 15%）",
                    "同时在托盘图标右上角显示微红角标",
                    cfg.low_battery_notify,
                    HitTarget::ToggleLowBatteryNotify,
                ),
                (
                    "开机静默自启",
                    "仅驻留系统托盘，不弹出窗口",
                    cfg.autostart,
                    HitTarget::ToggleAutostart,
                ),
            ];

            for (title, desc, checked, target) in toggles {
                let row_rect = rect_xywh(pad_x - 4.0, y, inner_w + 8.0, 44.0);
                let hovered = ui.hovered.as_ref() == Some(&target);
                if hovered {
                    let hb = rt.CreateSolidColorBrush(&palette.card_bg_hover, None)?;
                    rt.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: row_rect,
                            radiusX: 8.0,
                            radiusY: 8.0,
                        },
                        &hb,
                    );
                }

                draw_text(
                    rt,
                    title,
                    &self.formats.body,
                    rect_xywh(pad_x, y + 4.0, inner_w - 48.0, 18.0),
                    &text_primary,
                );
                draw_text(
                    rt,
                    desc,
                    &self.formats.caption,
                    rect_xywh(pad_x, y + 22.0, inner_w - 48.0, 16.0),
                    &text_subtle,
                );

                let sw_x = width - pad_x - 36.0;
                let sw_y = y + 12.0;
                let sw_rect = rect_xywh(sw_x, sw_y, 36.0, 20.0);
                let sw_bg = rt.CreateSolidColorBrush(
                    if checked {
                        &palette.accent_green
                    } else {
                        &palette.toggle_off
                    },
                    None,
                )?;
                let thumb_b = rt.CreateSolidColorBrush(&palette.toggle_thumb, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: sw_rect,
                        radiusX: 10.0,
                        radiusY: 10.0,
                    },
                    &sw_bg,
                );
                let thumb_cx = if checked { sw_x + 26.0 } else { sw_x + 10.0 };
                rt.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: D2D_POINT_2F {
                            x: thumb_cx,
                            y: sw_y + 10.0,
                        },
                        radiusX: 7.5,
                        radiusY: 7.5,
                    },
                    &thumb_b,
                );

                self.register_hit(row_rect, target);
                y += 48.0;
            }

            let _ = height;
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
        let hovered = ui.hovered == Some(HitTarget::TogglePin);
        unsafe {
            if ui.pinned {
                let bg = rt.CreateSolidColorBrush(&rgba(16, 185, 129, 0.18), None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect,
                        radiusX: 7.0,
                        radiusY: 7.0,
                    },
                    &bg,
                );
            } else if hovered {
                let bg = rt.CreateSolidColorBrush(&palette.pill_bg, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect,
                        radiusX: 7.0,
                        radiusY: 7.0,
                    },
                    &bg,
                );
            }
            let glyph_color = if ui.pinned {
                palette.accent_green
            } else if hovered {
                palette.text_primary
            } else {
                palette.text_secondary
            };
            let glyph_brush = rt.CreateSolidColorBrush(&glyph_color, None)?;
            let glyph = if ui.pinned { GLYPH_PINNED } else { GLYPH_PIN };
            draw_text(rt, glyph, &self.formats.icon_header, rect, &glyph_brush);
        }
        self.register_hit(rect, HitTarget::TogglePin);
        Ok(())
    }

    fn draw_glyph_button(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        rect: D2D_RECT_F,
        glyph: &str,
        target: &HitTarget,
        palette: &Palette,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        let hovered = ui.hovered.as_ref() == Some(target);
        unsafe {
            if hovered {
                let bg = rt.CreateSolidColorBrush(&palette.pill_bg, None)?;
                rt.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect,
                        radiusX: 7.0,
                        radiusY: 7.0,
                    },
                    &bg,
                );
            }
            let glyph_brush = rt.CreateSolidColorBrush(
                if hovered {
                    &palette.text_primary
                } else {
                    &palette.text_secondary
                },
                None,
            )?;
            draw_text(rt, glyph, &self.formats.icon_header, rect, &glyph_brush);
        }
        self.register_hit(rect, target.clone());
        Ok(())
    }

    fn draw_pill_button(
        &mut self,
        rt: &ID2D1HwndRenderTarget,
        rect: D2D_RECT_F,
        label: &str,
        target: HitTarget,
        palette: &Palette,
        ui: &UiState,
    ) -> windows::core::Result<()> {
        let hovered = ui.hovered.as_ref() == Some(&target);
        unsafe {
            let bg = rt.CreateSolidColorBrush(
                if hovered {
                    &palette.pill_bg_hover
                } else {
                    &palette.pill_bg
                },
                None,
            )?;
            let text_brush = rt.CreateSolidColorBrush(&palette.text_primary, None)?;
            rt.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect,
                    radiusX: 6.0,
                    radiusY: 6.0,
                },
                &bg,
            );
            draw_text(rt, label, &self.formats.body_center, rect, &text_brush);
        }
        self.register_hit(rect, target);
        Ok(())
    }

    fn register_hit(&mut self, rect: D2D_RECT_F, target: HitTarget) {
        self.hit_regions.push(HitRegion { rect, target });
    }
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

fn create_format(
    dwrite: &IDWriteFactory,
    font_family: PCWSTR,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    size: f32,
    align: windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT,
    para_align: windows::Win32::Graphics::DirectWrite::DWRITE_PARAGRAPH_ALIGNMENT,
    wrap: bool,
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
        return (palette.accent_green, palette.accent_green);
    }
    match dev.precision {
        Precision::Percentage => match dev.percentage {
            Some(pct) if pct <= 15 => (palette.accent_rose, palette.accent_rose),
            Some(pct) if pct <= 30 => (palette.accent_amber, palette.accent_amber),
            Some(_) if is_stale => (palette.text_secondary, rgba(16, 185, 129, 0.50)),
            Some(_) => (palette.text_primary, palette.accent_green),
            None => (palette.text_subtle, palette.bar_track),
        },
        Precision::DiscreteLevel => match dev.level {
            Some(BatteryLevel::Critical | BatteryLevel::Low) => {
                (palette.accent_amber, palette.accent_amber)
            }
            Some(_) => (palette.text_primary, palette.accent_green),
            None => (palette.text_subtle, palette.bar_track),
        },
        Precision::VoltageOnly => (palette.text_primary, palette.accent_sky),
        Precision::None => (palette.text_subtle, palette.bar_track),
    }
}

fn discrete_info(level: Option<BatteryLevel>) -> (u8, &'static str) {
    match level {
        Some(BatteryLevel::Critical) => (1, "极低"),
        Some(BatteryLevel::Low) => (1, "低"),
        Some(BatteryLevel::Medium) => (2, "中"),
        Some(BatteryLevel::High) => (3, "高"),
        Some(BatteryLevel::Full) => (4, "满"),
        None => (0, "未知"),
    }
}

fn error_kind_short_label(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Timeout => "读取超时",
        ErrorKind::DeviceBusy => "接口正忙",
        ErrorKind::PermissionDenied => "权限不足",
        ErrorKind::DeviceDisconnected => "连接已断开",
        ErrorKind::InvalidResponse | ErrorKind::ChecksumFailed => "响应校验失败",
        ErrorKind::Unsupported => "策略未启用",
        _ => "读取失败",
    }
}

fn format_device_subtitle(dev: &DeviceView) -> String {
    let mut parts: Vec<String> = Vec::new();
    if dev.charge_state == ChargeState::Charging {
        parts.push("充电中".to_owned());
    } else if dev.reachability == Reachability::Sleeping {
        parts.push("休眠".to_owned());
    } else if dev.reachability == Reachability::Offline {
        parts.push("离线".to_owned());
    } else if dev.precision == Precision::None {
        match dev.last_error {
            Some(ErrorKind::Timeout) => parts.push("读取超时 · 请唤醒后重试".to_owned()),
            Some(ErrorKind::DeviceBusy) => parts.push("接口正忙 · 稍后重试".to_owned()),
            Some(ErrorKind::PermissionDenied) => parts.push("权限不足".to_owned()),
            Some(ErrorKind::DeviceDisconnected) => parts.push("连接已断开".to_owned()),
            Some(_) => parts.push("读取失败 · 点此重读".to_owned()),
            None => parts.push(format!("{} · 读取中", dev.transport_label)),
        }
    } else if dev.precision == Precision::DiscreteLevel {
        parts.push("离散档位".to_owned());
    } else {
        parts.push(dev.transport_label.clone());
    }

    if dev.freshness == Some(Freshness::Stale) {
        parts.push(format!("缓存 · {}", format_relative_time(dev.observed_at)));
    } else if let Some(mv) = dev.voltage_mv {
        if dev.precision == Precision::Percentage {
            parts.push(format!("{:.2} V", f64::from(mv) / 1000.0));
        }
    }

    parts.join(" · ")
}

fn format_relative_time(at: Option<DateTime<Utc>>) -> String {
    let Some(ts) = at else {
        return "未同步".to_owned();
    };
    let elapsed = Utc::now().signed_duration_since(ts).num_seconds().max(0);
    if elapsed < 45 {
        "刚刚".to_owned()
    } else if elapsed < 3600 {
        format!("{} 分钟前", elapsed / 60)
    } else {
        format!("{} 小时前", elapsed / 3600)
    }
}
