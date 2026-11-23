//! Reusable Direct2D / DirectWrite UI primitives and cached brush helper.

use super::anim::{
    CommunityLinkKind, HitRegion, HitTarget, UiState, CARD_RADIUS, CONTROL_RADIUS, FOOTER_HEIGHT,
};
use crate::icon::resolve_icon_font_family;
use crate::theme::{lerp_color, with_alpha, Palette};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_END_CLOSED, D2D_POINT_2F, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush, D2D1_BEZIER_SEGMENT,
    D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory, IDWriteTextFormat, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_FONT_WEIGHT_REGULAR,
    DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_TEXT_ALIGNMENT,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWRITE_WORD_WRAPPING_WRAP,
};
use wireless_battery_tray_core::{DeviceKind, TraySnapshot};

// Built-in Windows `Segoe Fluent Icons` / `Segoe MDL2 Assets` codepoints.
pub(super) const GLYPH_PIN: &str = "\u{E718}";
pub(super) const GLYPH_PINNED: &str = "\u{E840}";
pub(super) const GLYPH_REFRESH: &str = "\u{E72C}";
pub(super) const GLYPH_SETTINGS: &str = "\u{E713}";
pub(super) const GLYPH_BACK: &str = "\u{E72B}";
pub(super) const GLYPH_CHEVRON_DOWN: &str = "\u{E70D}";
pub(super) const GLYPH_CHEVRON_UP: &str = "\u{E70E}";
pub(super) const GLYPH_COPY: &str = "\u{E8C8}";
pub(super) const GLYPH_CHECKMARK: &str = "\u{E73E}";
pub(super) const GLYPH_COLOR_THEME: &str = "\u{E790}";
pub(super) const GLYPH_LANGUAGE: &str = "\u{E774}";
pub(super) const GLYPH_CLOCK: &str = "\u{E823}";
pub(super) const GLYPH_RINGER: &str = "\u{EA8F}";
pub(super) const GLYPH_RINGER_SILENT: &str = "\u{E7ED}";
pub(super) const GLYPH_POWER: &str = "\u{E7E8}";
pub(super) const GLYPH_MOUSE: &str = "\u{E962}";
pub(super) const GLYPH_KEYBOARD: &str = "\u{E765}";
pub(super) const GLYPH_GAMEPAD: &str = "\u{E7FC}";
pub(super) const GLYPH_HEADSET: &str = "\u{E7F6}";
pub(super) const GLYPH_PERIPHERAL: &str = "\u{E857}";
pub(super) const GLYPH_CHARGING_BOLT: &str = "\u{E945}";
pub(super) const GLYPH_INFO: &str = "\u{E946}";
pub(super) const GLYPH_CODE: &str = "\u{E943}";
pub(super) const GLYPH_FEEDBACK: &str = "\u{ED15}";
pub(super) const GLYPH_PEOPLE: &str = "\u{E716}";

/// Single cached Direct2D solid color brush reused across draw calls on a render target.
#[derive(Clone)]
pub(super) struct PaintBrush(pub(super) ID2D1SolidColorBrush);

impl PaintBrush {
    pub(super) unsafe fn color(&self, c: &D2D1_COLOR_F) -> &ID2D1SolidColorBrush {
        self.0.SetColor(c);
        &self.0
    }
}

pub(super) struct TextFormats {
    pub(super) title: IDWriteTextFormat,
    pub(super) device_name: IDWriteTextFormat,
    pub(super) value_large: IDWriteTextFormat,
    pub(super) value_discrete: IDWriteTextFormat,
    pub(super) body: IDWriteTextFormat,
    pub(super) body_center: IDWriteTextFormat,
    pub(super) caption: IDWriteTextFormat,
    pub(super) caption_right: IDWriteTextFormat,
    pub(super) caption_center: IDWriteTextFormat,
    pub(super) mono_right: IDWriteTextFormat,
    pub(super) icon_header: IDWriteTextFormat,
    pub(super) icon_device: IDWriteTextFormat,
    pub(super) icon_setting: IDWriteTextFormat,
    pub(super) icon_small: IDWriteTextFormat,
    pub(super) icon_chevron: IDWriteTextFormat,
}

impl TextFormats {
    pub(super) fn create(dwrite: &IDWriteFactory) -> windows::core::Result<Self> {
        let icon_font = resolve_icon_font_family(dwrite);
        let ui_text_font =
            resolve_font_family(dwrite, w!("Segoe UI Variable Text"), w!("Segoe UI"));
        let ui_display_font =
            resolve_font_family(dwrite, w!("Segoe UI Variable Display"), w!("Segoe UI"));
        let ui_small_font =
            resolve_font_family(dwrite, w!("Segoe UI Variable Small"), w!("Segoe UI"));
        let mono_font = resolve_font_family(dwrite, w!("Cascadia Mono"), w!("Consolas"));

        Ok(Self {
            title: create_format(
                dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                15.0,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            device_name: create_format(
                dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                13.5,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
                true,
            )?,
            value_large: create_format(
                dwrite,
                ui_display_font,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                15.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
                false,
            )?,
            value_discrete: create_format(
                dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                13.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                false,
                false,
            )?,
            body: create_format(
                dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                12.5,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            body_center: create_format(
                dwrite,
                ui_text_font,
                DWRITE_FONT_WEIGHT_MEDIUM,
                12.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            caption: create_format(
                dwrite,
                ui_small_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.5,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            caption_right: create_format(
                dwrite,
                ui_small_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.5,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            caption_center: create_format(
                dwrite,
                ui_small_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                true,
                false,
            )?,
            mono_right: create_format(
                dwrite,
                mono_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                11.0,
                DWRITE_TEXT_ALIGNMENT_TRAILING,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                true,
            )?,
            icon_header: create_format(
                dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                14.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            icon_device: create_format(
                dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                17.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            icon_setting: create_format(
                dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                15.5,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            icon_small: create_format(
                dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                12.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
            icon_chevron: create_format(
                dwrite,
                icon_font,
                DWRITE_FONT_WEIGHT_REGULAR,
                10.0,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                false,
                false,
            )?,
        })
    }
}

pub(super) fn rect_xywh(x: f32, y: f32, w: f32, h: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}

pub(super) fn rounded_rect(rect: D2D_RECT_F, radius: f32) -> D2D1_ROUNDED_RECT {
    D2D1_ROUNDED_RECT {
        rect,
        radiusX: radius,
        radiusY: radius,
    }
}

pub(super) fn device_kind_glyph(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Mouse => GLYPH_MOUSE,
        DeviceKind::Keyboard => GLYPH_KEYBOARD,
        DeviceKind::Gamepad => GLYPH_GAMEPAD,
        DeviceKind::Headset => GLYPH_HEADSET,
        DeviceKind::Peripheral => GLYPH_PERIPHERAL,
    }
}

pub(super) fn estimate_text_width(text: &str) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { 7.0 } else { 12.5 })
        .sum()
}

pub(super) unsafe fn draw_text(
    rt: &ID2D1HwndRenderTarget,
    text: &str,
    format: &IDWriteTextFormat,
    rect: D2D_RECT_F,
    color: &D2D1_COLOR_F,
    brush: &PaintBrush,
) {
    let wide = HSTRING::from(text);
    rt.DrawText(
        wide.as_wide(),
        format,
        &rect,
        brush.color(color),
        D2D1_DRAW_TEXT_OPTIONS_CLIP,
        DWRITE_MEASURING_MODE_NATURAL,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_text_shadowed(
    rt: &ID2D1HwndRenderTarget,
    text: &str,
    format: &IDWriteTextFormat,
    rect: D2D_RECT_F,
    color: &D2D1_COLOR_F,
    shadow: Option<&D2D1_COLOR_F>,
    brush: &PaintBrush,
) {
    if let Some(sc) = shadow {
        let s_rect = D2D_RECT_F {
            left: rect.left,
            top: rect.top + 1.0,
            right: rect.right,
            bottom: rect.bottom + 1.0,
        };
        draw_text(rt, text, format, s_rect, sc, brush);
    }
    draw_text(rt, text, format, rect, color, brush);
}

/// Draw a WinUI 3 surface card/button with 1px `ControlStrokeColorDefault` border and
/// subtle top (dark mode) or bottom (light mode) elevation bevel highlight.
#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_elevated_card(
    rt: &ID2D1HwndRenderTarget,
    brush: &PaintBrush,
    rect: D2D_RECT_F,
    radius: f32,
    fill: D2D1_COLOR_F,
    border: D2D1_COLOR_F,
    palette: &Palette,
) {
    let rr = rounded_rect(
        D2D_RECT_F {
            left: rect.left + 0.5,
            top: rect.top + 0.5,
            right: rect.right - 0.5,
            bottom: rect.bottom - 0.5,
        },
        radius,
    );
    rt.FillRoundedRectangle(&rr, brush.color(&fill));
    rt.DrawRoundedRectangle(&rr, brush.color(&border), 1.0, None);

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
        brush.color(&palette.elevation_edge),
        1.0,
        None,
    );
}

/// Draw an authentic animated Windows 11 Fluent 2 `ToggleSwitch` (40x20 DIPs).
#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_win11_toggle_switch(
    rt: &ID2D1HwndRenderTarget,
    brush: &PaintBrush,
    x: f32,
    y: f32,
    t: f32,
    hovered: bool,
    pressed: bool,
    palette: &Palette,
) {
    let t = t.clamp(0.0, 1.0);
    let rr = rounded_rect(rect_xywh(x + 0.5, y + 0.5, 39.0, 19.0), 9.5);

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
    rt.FillRoundedRectangle(&rr, brush.color(&track_color));

    if t < 0.99 {
        let border_color = with_alpha(palette.toggle_off_border, 1.0 - t);
        rt.DrawRoundedRectangle(&rr, brush.color(&border_color), 1.0, None);
    }

    let thumb_h = if pressed || hovered { 14.0 } else { 12.0 };
    let thumb_w = if pressed { 17.0 } else { thumb_h };
    let left_x = x + 4.0;
    let right_x = x + 36.0 - thumb_w;
    let thumb_x = left_x + (right_x - left_x) * t;
    let thumb_y = y + (20.0 - thumb_h) * 0.5;

    let thumb_color = lerp_color(palette.toggle_off_thumb, palette.text_on_accent, t);
    rt.FillRoundedRectangle(
        &rounded_rect(rect_xywh(thumb_x, thumb_y, thumb_w, thumb_h), thumb_h * 0.5),
        brush.color(&thumb_color),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_subtle_icon_button(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    rect: D2D_RECT_F,
    glyph: &str,
    target: &HitTarget,
    active: bool,
    palette: &Palette,
    ui: &UiState,
) {
    let hovered = ui.hovered.as_ref() == Some(target);
    let pressed = ui.pressed.as_ref() == Some(target);
    let rr = rounded_rect(rect, CONTROL_RADIUS);
    if active {
        rt.FillRoundedRectangle(&rr, brush.color(&palette.accent_subtle_bg));
        rt.DrawRoundedRectangle(&rr, brush.color(&palette.accent_subtle_border), 1.0, None);
    } else if pressed || hovered {
        let fill = if pressed {
            &palette.subtle_pressed
        } else {
            &palette.subtle_hover
        };
        rt.FillRoundedRectangle(&rr, brush.color(fill));
    }

    let glyph_color = if active {
        palette.accent_text
    } else if hovered || pressed {
        palette.text_primary
    } else {
        palette.text_secondary
    };
    draw_text(rt, glyph, &formats.icon_header, rect, &glyph_color, brush);
    hit_regions.push(HitRegion {
        rect,
        target: target.clone(),
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_standard_button(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    rect: D2D_RECT_F,
    glyph: Option<&str>,
    label: &str,
    target: HitTarget,
    palette: &Palette,
    ui: &UiState,
    alpha: f32,
) {
    let hovered = ui.hovered.as_ref() == Some(&target);
    let pressed = ui.pressed.as_ref() == Some(&target);
    let bg_color = if pressed {
        palette.control_bg_pressed
    } else if hovered {
        palette.control_bg_hover
    } else {
        palette.control_bg
    };
    draw_elevated_card(
        rt,
        brush,
        rect,
        CONTROL_RADIUS,
        with_alpha(bg_color, alpha),
        with_alpha(palette.control_border, alpha),
        palette,
    );

    let txt_color = if pressed {
        with_alpha(palette.text_secondary, alpha)
    } else {
        with_alpha(palette.text_primary, alpha)
    };

    if let Some(icon_glyph) = glyph {
        let h = rect.bottom - rect.top;
        let total_w = 13.0 + 5.0 + estimate_text_width(label);
        let start_x = ((rect.left + rect.right - total_w) * 0.5).max(rect.left + 8.0);
        draw_text(
            rt,
            icon_glyph,
            &formats.icon_small,
            rect_xywh(start_x, rect.top, 13.0, h),
            &txt_color,
            brush,
        );
        draw_text(
            rt,
            label,
            &formats.body,
            rect_xywh(
                start_x + 18.0,
                rect.top,
                (rect.right - start_x - 22.0).max(20.0),
                h,
            ),
            &txt_color,
            brush,
        );
    } else {
        draw_text(rt, label, &formats.body_center, rect, &txt_color, brush);
    }
    hit_regions.push(HitRegion { rect, target });
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_toast(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    width: f32,
    height: f32,
    palette: &Palette,
    snapshot: &TraySnapshot,
    ui: &UiState,
    msg: &str,
) {
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
    let rr = rounded_rect(rect, CARD_RADIUS);
    rt.FillRoundedRectangle(&rr, brush.color(&palette.toast_bg));
    rt.DrawRoundedRectangle(&rr, brush.color(&palette.toast_border), 1.0, None);
    draw_text(
        rt,
        GLYPH_CHECKMARK,
        &formats.icon_small,
        rect_xywh(rect.left + 12.0, rect.top, 16.0, banner_h),
        &palette.status_good,
        brush,
    );
    draw_text(
        rt,
        msg,
        &formats.body,
        rect_xywh(rect.left + 32.0, rect.top, est_w - 42.0, banner_h),
        &palette.toast_text,
        brush,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_community_link_button(
    d2d: &ID2D1Factory,
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    rect: D2D_RECT_F,
    kind: CommunityLinkKind,
    palette: &Palette,
    ui: &UiState,
) {
    let target = HitTarget::OpenCommunityLink(kind);
    let hovered = ui.hovered.as_ref() == Some(&target);
    let pressed = ui.pressed.as_ref() == Some(&target);
    let rr = rounded_rect(rect, CONTROL_RADIUS);
    if pressed || hovered {
        let fill = if pressed {
            &palette.subtle_pressed
        } else {
            &palette.subtle_hover
        };
        rt.FillRoundedRectangle(&rr, brush.color(fill));
    }

    let icon_color = if hovered || pressed {
        palette.text_primary
    } else {
        palette.text_secondary
    };
    let icon_size = 12.0;
    let icon_rect = rect_xywh(
        rect.left + (rect.right - rect.left - icon_size) * 0.5,
        rect.top + (rect.bottom - rect.top - icon_size) * 0.5,
        icon_size,
        icon_size,
    );
    draw_community_icon(d2d, rt, formats, brush, kind, icon_rect, &icon_color);
    hit_regions.push(HitRegion { rect, target });
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_hover_tooltip(
    d2d: &ID2D1Factory,
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    width: f32,
    list_bottom: f32,
    palette: &Palette,
    kind: CommunityLinkKind,
    text: &str,
) {
    let est_w = (estimate_text_width(text) + 46.0).clamp(132.0, width - 24.0);
    let pill_h = 30.0;
    let rect = D2D_RECT_F {
        left: (width - est_w - 12.0).max(12.0),
        top: list_bottom - pill_h - 8.0,
        right: width - 12.0,
        bottom: list_bottom - 8.0,
    };
    let rr = rounded_rect(rect, CARD_RADIUS);
    rt.FillRoundedRectangle(&rr, brush.color(&palette.toast_bg));
    rt.DrawRoundedRectangle(&rr, brush.color(&palette.toast_border), 1.0, None);

    let icon_rect = rect_xywh(rect.left + 11.0, rect.top + (pill_h - 11.0) * 0.5, 11.0, 11.0);
    draw_community_icon(
        d2d,
        rt,
        formats,
        brush,
        kind,
        icon_rect,
        &palette.text_secondary,
    );
    draw_text(
        rt,
        text,
        &formats.caption,
        rect_xywh(rect.left + 28.0, rect.top, est_w - 36.0, pill_h),
        &palette.toast_text,
        brush,
    );
}

unsafe fn draw_community_icon(
    d2d: &ID2D1Factory,
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    kind: CommunityLinkKind,
    rect: D2D_RECT_F,
    color: &D2D1_COLOR_F,
) {
    match kind {
        CommunityLinkKind::BugReport => {
            draw_text(
                rt,
                GLYPH_FEEDBACK,
                &formats.icon_small,
                rect_xywh(rect.left - 1.0, rect.top - 1.0, 14.0, 14.0),
                color,
                brush,
            );
        }
        CommunityLinkKind::SourceRepo => {
            let sx = (rect.right - rect.left) / 24.0;
            let sy = (rect.bottom - rect.top) / 24.0;
            let ox = rect.left;
            let oy = rect.top;
            let pt = |x: f32, y: f32| D2D_POINT_2F {
                x: ox + x * sx,
                y: oy + y * sy,
            };
            if let Ok(geo) = d2d.CreatePathGeometry() {
                if let Ok(sink) = geo.Open() {
                    sink.BeginFigure(pt(12.0, 1.5), D2D1_FIGURE_BEGIN_FILLED);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.2, 1.5),
                        point2: pt(1.5, 6.2),
                        point3: pt(1.5, 12.0),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(1.5, 16.64),
                        point2: pt(4.51, 20.57),
                        point3: pt(8.68, 21.96),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(9.21, 22.06),
                        point2: pt(9.38, 21.74),
                        point3: pt(9.38, 21.31),
                    });
                    sink.AddLine(pt(9.38, 19.01));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.46, 19.64),
                        point2: pt(5.84, 17.6),
                        point3: pt(6.13, 17.79),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(5.65, 16.58),
                        point2: pt(4.96, 16.26),
                        point3: pt(4.97, 16.32),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(4.02, 15.67),
                        point2: pt(5.04, 15.68),
                        point3: pt(5.05, 16.1),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.1, 16.17),
                        point2: pt(6.66, 17.18),
                        point3: pt(6.66, 17.18),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(7.6, 18.78),
                        point2: pt(9.12, 18.32),
                        point3: pt(9.43, 18.08),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(9.52, 17.4),
                        point2: pt(9.8, 16.94),
                        point3: pt(10.17, 16.65),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(7.84, 16.39),
                        point2: pt(5.39, 15.49),
                        point3: pt(5.39, 11.46),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(5.39, 10.31),
                        point2: pt(5.8, 9.37),
                        point3: pt(6.46, 8.63),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.35, 8.37),
                        point2: pt(6.0, 7.31),
                        point3: pt(6.56, 5.84),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.56, 5.84),
                        point2: pt(7.43, 5.56),
                        point3: pt(9.45, 6.94),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(11.12, 6.48),
                        point2: pt(12.88, 6.48),
                        point3: pt(14.55, 6.94),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(16.57, 5.56),
                        point2: pt(17.44, 5.84),
                        point3: pt(17.44, 5.84),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(18.0, 7.31),
                        point2: pt(17.65, 8.37),
                        point3: pt(17.54, 8.63),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(18.2, 9.37),
                        point2: pt(18.61, 10.31),
                        point3: pt(18.61, 11.46),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(18.61, 15.5),
                        point2: pt(16.15, 16.38),
                        point3: pt(13.82, 16.64),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(14.29, 17.04),
                        point2: pt(14.62, 17.78),
                        point3: pt(14.62, 18.61),
                    });
                    sink.AddLine(pt(14.62, 21.31));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(14.62, 21.75),
                        point2: pt(14.79, 22.06),
                        point3: pt(15.32, 21.96),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(19.49, 20.57),
                        point2: pt(22.5, 16.64),
                        point3: pt(22.5, 12.0),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(22.5, 6.2),
                        point2: pt(17.8, 1.5),
                        point3: pt(12.0, 1.5),
                    });
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    let _ = sink.Close();
                    rt.FillGeometry(&geo, brush.color(color), None);
                }
            }
        }
        CommunityLinkKind::Qq => {
            let sx = (rect.right - rect.left) / 24.0;
            let sy = (rect.bottom - rect.top) / 24.0;
            let ox = rect.left;
            let oy = rect.top;
            let pt = |x: f32, y: f32| D2D_POINT_2F {
                x: ox + x * sx,
                y: oy + y * sy,
            };
            if let Ok(geo) = d2d.CreatePathGeometry() {
                if let Ok(sink) = geo.Open() {
                    sink.BeginFigure(pt(12.0, 2.0), D2D1_FIGURE_BEGIN_FILLED);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(8.4, 2.0),
                        point2: pt(6.4, 5.0),
                        point3: pt(6.4, 8.9),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(5.1, 10.6),
                        point2: pt(4.0, 12.9),
                        point3: pt(4.1, 14.6),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(4.2, 16.0),
                        point2: pt(5.2, 16.5),
                        point3: pt(5.9, 16.0),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.3, 17.2),
                        point2: pt(6.8, 18.1),
                        point3: pt(7.4, 18.8),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.3, 19.3),
                        point2: pt(5.6, 20.0),
                        point3: pt(5.8, 20.6),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.1, 21.5),
                        point2: pt(8.9, 21.6),
                        point3: pt(10.8, 21.0),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(11.6, 21.1),
                        point2: pt(12.4, 21.1),
                        point3: pt(13.2, 21.0),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(15.1, 21.6),
                        point2: pt(17.9, 21.5),
                        point3: pt(18.2, 20.6),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(18.4, 20.0),
                        point2: pt(17.7, 19.3),
                        point3: pt(16.6, 18.8),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(17.2, 18.1),
                        point2: pt(17.7, 17.2),
                        point3: pt(18.1, 16.0),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(18.8, 16.5),
                        point2: pt(19.8, 16.0),
                        point3: pt(19.9, 14.6),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(20.0, 12.9),
                        point2: pt(18.9, 10.6),
                        point3: pt(17.6, 8.9),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(17.6, 5.0),
                        point2: pt(15.6, 2.0),
                        point3: pt(12.0, 2.0),
                    });
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    let _ = sink.Close();
                    rt.FillGeometry(&geo, brush.color(color), None);
                }
            }
        }
        CommunityLinkKind::Discord => {
            let w = rect.right - rect.left;
            let h = w * 0.78;
            let sx = w / 24.0;
            let sy = h / 18.0;
            let ox = rect.left;
            let oy = rect.top + (rect.bottom - rect.top - h) * 0.5;
            let pt = |x: f32, y: f32| D2D_POINT_2F {
                x: ox + x * sx,
                y: oy + y * sy,
            };
            if let Ok(geo) = d2d.CreatePathGeometry() {
                if let Ok(sink) = geo.Open() {
                    sink.BeginFigure(pt(20.32, 1.5), D2D1_FIGURE_BEGIN_FILLED);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(18.82, 0.81),
                        point2: pt(17.24, 0.32),
                        point3: pt(15.63, 0.05),
                    });
                    sink.AddLine(pt(15.0, 1.32));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(13.02, 1.02),
                        point2: pt(10.98, 1.02),
                        point3: pt(9.0, 1.32),
                    });
                    sink.AddLine(pt(8.37, 0.05));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.76, 0.32),
                        point2: pt(5.18, 0.81),
                        point3: pt(3.68, 1.5),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(0.72, 5.91),
                        point2: pt(-0.09, 10.22),
                        point3: pt(0.56, 14.37),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(2.26, 15.63),
                        point2: pt(4.18, 16.61),
                        point3: pt(6.32, 17.28),
                    });
                    sink.AddLine(pt(7.55, 15.27));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(6.88, 15.02),
                        point2: pt(6.23, 14.71),
                        point3: pt(5.61, 14.33),
                    });
                    sink.AddLine(pt(6.09, 13.96));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(9.85, 15.69),
                        point2: pt(14.15, 15.69),
                        point3: pt(17.91, 13.96),
                    });
                    sink.AddLine(pt(18.39, 14.33));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(17.77, 14.71),
                        point2: pt(17.12, 15.02),
                        point3: pt(16.45, 15.27),
                    });
                    sink.AddLine(pt(17.68, 17.28));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(19.82, 16.61),
                        point2: pt(21.74, 15.63),
                        point3: pt(23.44, 14.37),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(24.2, 9.57),
                        point2: pt(22.74, 5.3),
                        point3: pt(20.32, 1.5),
                    });
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);

                    // Eye cutouts (even-odd fill)
                    for cx in [8.1_f32, 15.9_f32] {
                        let cy = 9.3_f32;
                        let rx = 1.7_f32;
                        let ry = 1.9_f32;
                        let kx = rx * 0.552;
                        let ky = ry * 0.552;
                        sink.BeginFigure(pt(cx, cy - ry), D2D1_FIGURE_BEGIN_FILLED);
                        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                            point1: pt(cx + kx, cy - ry),
                            point2: pt(cx + rx, cy - ky),
                            point3: pt(cx + rx, cy),
                        });
                        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                            point1: pt(cx + rx, cy + ky),
                            point2: pt(cx + kx, cy + ry),
                            point3: pt(cx, cy + ry),
                        });
                        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                            point1: pt(cx - kx, cy + ry),
                            point2: pt(cx - rx, cy + ky),
                            point3: pt(cx - rx, cy),
                        });
                        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                            point1: pt(cx - rx, cy - ky),
                            point2: pt(cx - kx, cy - ry),
                            point3: pt(cx, cy - ry),
                        });
                        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    }
                    let _ = sink.Close();
                    rt.FillGeometry(&geo, brush.color(color), None);
                }
            }
        }
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
    weight: DWRITE_FONT_WEIGHT,
    size: f32,
    align: DWRITE_TEXT_ALIGNMENT,
    para_align: DWRITE_PARAGRAPH_ALIGNMENT,
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
