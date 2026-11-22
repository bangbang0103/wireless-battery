//! Direct2D + DirectWrite renderer and hit-testing engine for the Windows 11 Fluent 2 popover.
//!
//! Aligned to official WinUI 3 / Windows 11 Quick Settings & Settings metrics:
//! - Main flyout: seamless divider-separated device rows (`1px` hairline between devices),
//!   minimalist `52px` header & `48px` footer; compact pinned mode becomes 100% background-free.
//! - Settings view: flat WinUI 3 `SettingsCard` rows (`4px` gap) using native-style
//!   `ComboBox` dropdown flyouts, Low Battery Alert with indented Silent Notifications sub-option,
//!   Experimental Protocol toggle, and Launch at Startup.
//! - Bilingual Simplified Chinese & English UI (`i18n::Lang`).
//! - Silky DWM-VSync-paced micro-animations (60/120/144/240Hz lock-step, 0% idle CPU).

mod anim;
mod main_view;
mod settings_view;
mod widgets;

use self::anim::{
    HitRegion, FOOTER_HEIGHT, LIST_VERTICAL_PAD, OUTER_PADDING, POPOVER_MAX_HEIGHT_DIP,
    ROW_COLLAPSED_H, ROW_EXPANDED_EXTRA_H, SETTINGS_CARD_GAP, SETTINGS_CARD_H, SETTINGS_SUBROW_H,
    WINDOW_CORNER_RADIUS,
};
use self::widgets::{draw_toast, rounded_rect, PaintBrush, TextFormats};
use crate::i18n::Lang;
use crate::theme::{Palette, SystemTheme};
use std::time::Instant;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_IMMEDIATELY,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
    D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use wireless_battery_tray_core::TraySnapshot;

pub use self::anim::{
    HitTarget, PopoverView, UiState, UpdateUiStatus, HEADER_HEIGHT, POPOVER_WIDTH_DIP,
};

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

pub struct PopoverRenderer {
    d2d: ID2D1Factory,
    formats: TextFormats,
    target: Option<ID2D1HwndRenderTarget>,
    brush: Option<PaintBrush>,
    hit_regions: Vec<HitRegion>,
    content_height: f32,
    viewport_height: f32,
}

impl PopoverRenderer {
    pub fn new() -> windows::core::Result<Self> {
        let d2d: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
        let dwrite: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let formats = TextFormats::create(&dwrite)?;

        Ok(Self {
            d2d,
            formats,
            target: None,
            brush: None,
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
                    + SETTINGS_CARD_H * 7.0
                    + SETTINGS_CARD_GAP * 6.0
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
        let (rt, brush) = self.ensure_target(
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
                rt.FillRectangle(
                    &D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: width,
                        bottom: height,
                    },
                    brush.color(&bg_color),
                );
            }

            match ui.view {
                PopoverView::Main => {
                    main_view::draw_main_view(
                        &rt,
                        &self.formats,
                        &brush,
                        &mut self.hit_regions,
                        &mut self.content_height,
                        &mut self.viewport_height,
                        width,
                        height,
                        &palette,
                        lang,
                        snapshot,
                        ui,
                        now,
                    );
                }
                PopoverView::Settings => {
                    settings_view::draw_settings_view(
                        &rt,
                        &self.formats,
                        &brush,
                        &mut self.hit_regions,
                        width,
                        height,
                        &palette,
                        lang,
                        surface.system_theme,
                        snapshot,
                        ui,
                        now,
                    );
                }
            }

            if let Some(msg) = &ui.feedback_banner {
                draw_toast(
                    &rt,
                    &self.formats,
                    &brush,
                    width,
                    height,
                    &palette,
                    snapshot,
                    ui,
                    msg,
                );
            }

            if !compact {
                rt.DrawRoundedRectangle(
                    &rounded_rect(
                        D2D_RECT_F {
                            left: 0.5,
                            top: 0.5,
                            right: width - 0.5,
                            bottom: height - 0.5,
                        },
                        WINDOW_CORNER_RADIUS - 0.5,
                    ),
                    brush.color(&palette.outer_border),
                    1.0,
                    None,
                );
            }

            let end_result = rt.EndDraw(None, None);
            if end_result.is_err() {
                self.target = None;
                self.brush = None;
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
    ) -> windows::core::Result<(ID2D1HwndRenderTarget, PaintBrush)> {
        let size = D2D_SIZE_U {
            width: pixel_width.max(1),
            height: pixel_height.max(1),
        };
        if let Some(existing) = &self.target {
            unsafe {
                existing.SetDpi(dpi as f32, dpi as f32);
                let _ = existing.Resize(&size);
            }
            let brush = match &self.brush {
                Some(b) => b.clone(),
                None => {
                    let b = PaintBrush(unsafe {
                        existing.CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)?
                    });
                    self.brush = Some(b.clone());
                    b
                }
            };
            return Ok((existing.clone(), brush));
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
        let brush =
            PaintBrush(unsafe { target.CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)? });
        self.target = Some(target.clone());
        self.brush = Some(brush.clone());
        Ok((target, brush))
    }
}
