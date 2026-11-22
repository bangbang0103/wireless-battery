//! Main device list, header, footer, empty state, and expandable diagnostic drawer rendering.

use super::anim::{
    HitRegion, HitTarget, UiState, CARD_RADIUS, CONTROL_RADIUS, FOOTER_HEIGHT, HEADER_HEIGHT,
    LIST_VERTICAL_PAD, OUTER_PADDING, ROW_COLLAPSED_H, ROW_EXPANDED_EXTRA_H, WINDOW_CORNER_RADIUS,
};
use super::widgets::{
    device_kind_glyph, draw_standard_button, draw_subtle_icon_button, draw_text,
    draw_text_shadowed, rect_xywh, rounded_rect, PaintBrush, TextFormats, GLYPH_CHARGING_BOLT,
    GLYPH_CHEVRON_DOWN, GLYPH_CHEVRON_UP, GLYPH_COPY, GLYPH_PERIPHERAL, GLYPH_PIN, GLYPH_PINNED,
    GLYPH_REFRESH, GLYPH_SETTINGS,
};
use crate::i18n::Lang;
use crate::theme::{with_alpha, Palette};
use std::time::Instant;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_POINT_2F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{
    ID2D1HwndRenderTarget, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_ELLIPSE, D2D1_ROUNDED_RECT,
};
use wireless_battery::{BatteryLevel, ChargeState, Freshness, Precision, Reachability};
use wireless_battery_tray_core::{DeviceView, TrayIconState, TraySnapshot};

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_main_view(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    content_height: &mut f32,
    viewport_height: &mut f32,
    width: f32,
    height: f32,
    palette: &Palette,
    lang: Lang,
    snapshot: &TraySnapshot,
    ui: &mut UiState,
    now: Instant,
) {
    let compact = ui.is_compact_pinned(snapshot);
    if !compact {
        // --- Win11 Flyout Header (52 DIPs) ---
        draw_text(
            rt,
            "Wireless Battery",
            &formats.title,
            rect_xywh(OUTER_PADDING, 10.0, width - 136.0, 32.0),
            &palette.text_primary,
            brush,
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

        let pin_glyph = if ui.pinned { GLYPH_PINNED } else { GLYPH_PIN };
        draw_subtle_icon_button(
            rt,
            formats,
            brush,
            hit_regions,
            pin_rect,
            pin_glyph,
            &HitTarget::TogglePin,
            ui.pinned,
            palette,
            ui,
        );
        draw_subtle_icon_button(
            rt,
            formats,
            brush,
            hit_regions,
            refresh_rect,
            GLYPH_REFRESH,
            &HitTarget::RefreshAll,
            false,
            palette,
            ui,
        );
        draw_subtle_icon_button(
            rt,
            formats,
            brush,
            hit_regions,
            settings_rect,
            GLYPH_SETTINGS,
            &HitTarget::OpenSettings,
            false,
            palette,
            ui,
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
            brush.color(&palette.divider),
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
            rt.PushAxisAlignedClip(
                &rect_xywh(0.0, HEADER_HEIGHT - 2.5, width, 3.0),
                D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            );
            rt.FillRoundedRectangle(
                &rounded_rect(rect_xywh(seg_x, HEADER_HEIGHT - 2.0, seg_w, 2.0), 1.0),
                brush.color(&palette.accent_fill),
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
    *viewport_height = (list_bottom - list_top).max(1.0);

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
        *content_height = *viewport_height;
        ui.scroll_offset = 0.0;
        draw_empty_state(
            rt,
            formats,
            brush,
            hit_regions,
            D2D_RECT_F {
                left: 0.0,
                top: list_top,
                right: width,
                bottom: list_bottom,
            },
            palette,
            lang,
            ui,
        );
    } else {
        let mut total_h = LIST_VERTICAL_PAD * 2.0;
        for dev in &snapshot.devices {
            total_h += ROW_COLLAPSED_H;
            if !compact && ui.expanded_device_id.as_deref() == Some(dev.id.as_str()) {
                total_h += ROW_EXPANDED_EXTRA_H;
            }
        }
        *content_height = total_h;
        let max_scroll = (*content_height - *viewport_height).max(0.0);
        ui.scroll_offset = ui.scroll_offset.clamp(0.0, max_scroll);

        let has_scroll = max_scroll > 0.5;
        let mut y = list_top + LIST_VERTICAL_PAD - ui.scroll_offset;
        let reveal = ui.drawer_reveal_anim.value(now);
        let row_div_color = if compact {
            with_alpha(palette.divider, 1.4)
        } else {
            palette.divider
        };

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
                    brush.color(&row_div_color),
                    1.0,
                    None,
                );
            }

            let expanded = !compact && ui.expanded_device_id.as_deref() == Some(dev.id.as_str());
            let row_h = if expanded {
                ROW_COLLAPSED_H + ROW_EXPANDED_EXTRA_H
            } else {
                ROW_COLLAPSED_H
            };
            let row_rect = rect_xywh(0.0, y, width, row_h);
            if row_rect.bottom >= list_top - 4.0 && row_rect.top <= list_bottom + 4.0 {
                draw_device_row(
                    rt,
                    formats,
                    brush,
                    hit_regions,
                    row_rect,
                    dev,
                    expanded,
                    compact,
                    reveal,
                    palette,
                    lang,
                    ui,
                );
            }
            y += row_h;
        }

        if has_scroll && !compact {
            let track_top = list_top + 10.0;
            let track_h = (list_bottom - list_top - 20.0).max(20.0);
            let thumb_h = ((*viewport_height / *content_height) * track_h).clamp(28.0, track_h);
            let scroll_ratio = ui.scroll_offset / max_scroll;
            let thumb_y = track_top + scroll_ratio * (track_h - thumb_h);
            rt.FillRoundedRectangle(
                &rounded_rect(rect_xywh(width - 6.0, thumb_y, 3.0, thumb_h), 1.5),
                brush.color(&palette.scrollbar_thumb),
            );
        }
    }

    rt.PopAxisAlignedClip();

    if !compact {
        draw_footer(
            rt,
            formats,
            brush,
            hit_regions,
            list_bottom,
            width,
            height,
            palette,
            lang,
            snapshot,
            ui,
        );
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_footer(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    list_bottom: f32,
    width: f32,
    height: f32,
    palette: &Palette,
    lang: Lang,
    snapshot: &TraySnapshot,
    ui: &UiState,
) {
    rt.PushAxisAlignedClip(
        &D2D_RECT_F {
            left: 0.0,
            top: list_bottom,
            right: width,
            bottom: height,
        },
        D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
    );
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
        brush.color(&palette.footer_bg),
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
        brush.color(&palette.divider),
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
        brush.color(&dot_color),
    );

    let footer_status = if snapshot.refreshing {
        format!("{} · {}", snapshot.app_version, lang.reading_peripherals())
    } else {
        lang.footer_summary(
            &snapshot.app_version,
            snapshot.last_sync_at,
            snapshot.config.poll_interval_secs,
        )
    };
    draw_text(
        rt,
        &footer_status,
        &formats.caption,
        rect_xywh(
            OUTER_PADDING + 13.0,
            list_bottom,
            width - 96.0,
            FOOTER_HEIGHT,
        ),
        &palette.text_secondary,
        brush,
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
        rt.FillRoundedRectangle(&rounded_rect(quit_rect, CONTROL_RADIUS), brush.color(fill));
    }
    draw_text(
        rt,
        lang.quit(),
        &formats.body_center,
        quit_rect,
        if quit_hovered || quit_pressed {
            &palette.text_primary
        } else {
            &palette.text_secondary
        },
        brush,
    );
    hit_regions.push(HitRegion {
        rect: quit_rect,
        target: HitTarget::Quit,
    });
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_empty_state(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    bounds: D2D_RECT_F,
    palette: &Palette,
    lang: Lang,
    ui: &UiState,
) {
    let width = bounds.right - bounds.left;
    let center_y = bounds.top + (bounds.bottom - bounds.top) * 0.5;
    let well_rect = rect_xywh((width - 44.0) * 0.5, center_y - 70.0, 44.0, 44.0);

    rt.FillRoundedRectangle(
        &rounded_rect(well_rect, CARD_RADIUS),
        brush.color(&palette.icon_well),
    );
    draw_text(
        rt,
        GLYPH_PERIPHERAL,
        &formats.icon_device,
        well_rect,
        &palette.text_secondary,
        brush,
    );
    draw_text(
        rt,
        lang.empty_title(),
        &formats.body_center,
        rect_xywh(24.0, center_y - 16.0, width - 48.0, 22.0),
        &palette.text_primary,
        brush,
    );

    let hint = lang.empty_hint();
    draw_text(
        rt,
        &hint,
        &formats.caption_center,
        rect_xywh(36.0, center_y + 10.0, width - 72.0, 36.0),
        &palette.text_secondary,
        brush,
    );

    let btn_rect = rect_xywh((width - 112.0) * 0.5, center_y + 54.0, 112.0, 30.0);
    draw_standard_button(
        rt,
        formats,
        brush,
        hit_regions,
        btn_rect,
        None,
        lang.empty_action(),
        HitTarget::RefreshAll,
        palette,
        ui,
        1.0,
    );
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_device_row(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    rect: D2D_RECT_F,
    dev: &DeviceView,
    expanded: bool,
    compact: bool,
    drawer_alpha: f32,
    palette: &Palette,
    lang: Lang,
    ui: &UiState,
) {
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
        rt.FillRoundedRectangle(&rounded_rect(header_hit_rect, 6.0), brush.color(fill));
    }

    if !compact {
        hit_regions.push(HitRegion {
            rect: rect_xywh(
                rect.left + 8.0,
                rect.top,
                (rect.right - rect.left) - 16.0,
                ROW_COLLAPSED_H,
            ),
            target: toggle_target,
        });
    }

    let shadow_color = compact.then_some(&palette.compact_text_shadow);
    let well_rect = rect_xywh(rect.left + OUTER_PADDING, rect.top + 15.0, 36.0, 36.0);
    if !compact {
        rt.FillRoundedRectangle(
            &rounded_rect(well_rect, CARD_RADIUS),
            brush.color(&palette.icon_well),
        );
    }
    draw_text_shadowed(
        rt,
        device_kind_glyph(dev.kind),
        &formats.icon_device,
        well_rect,
        &palette.icon_stroke,
        shadow_color,
        brush,
    );

    let is_stale = dev.freshness == Some(Freshness::Stale);
    let is_offline = dev.reachability == Reachability::Offline;
    let name_color = if is_offline {
        &palette.text_secondary
    } else {
        &palette.text_primary
    };
    let sub_color = if dev.charge_state == ChargeState::Charging {
        &palette.status_good
    } else if dev.precision == Precision::None && dev.last_error.is_some() {
        &palette.status_caution
    } else {
        &palette.text_secondary
    };

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
        &formats.device_name,
        rect_xywh(text_left, rect.top + 14.0, text_width, 20.0),
        name_color,
        shadow_color,
        brush,
    );

    let subtitle = lang.format_device_subtitle(dev);
    draw_text_shadowed(
        rt,
        &subtitle,
        &formats.caption,
        rect_xywh(text_left, rect.top + 35.0, text_width, 18.0),
        sub_color,
        shadow_color,
        brush,
    );

    let bar_y = rect.top + 42.0;
    let (val_color, bar_color) = battery_tone(dev, palette);

    match dev.precision {
        Precision::Percentage => {
            if let Some(pct) = dev.percentage {
                if dev.charge_state == ChargeState::Charging {
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
                        &formats.icon_small,
                        rect_xywh(bolt_x, rect.top + 16.0, 12.0, 16.0),
                        &palette.status_good,
                        shadow_color,
                        brush,
                    );
                }
                let label = format!("{pct}%");
                draw_text_shadowed(
                    rt,
                    &label,
                    &formats.value_large,
                    rect_xywh(right_x, rect.top + 14.0, right_w, 21.0),
                    &val_color,
                    shadow_color,
                    brush,
                );
                rt.FillRoundedRectangle(
                    &rounded_rect(rect_xywh(right_x, bar_y, right_w, 4.0), 2.0),
                    brush.color(&palette.bar_track),
                );
                let fill_w = (right_w * (pct as f32 / 100.0)).clamp(4.0, right_w);
                rt.FillRoundedRectangle(
                    &rounded_rect(rect_xywh(right_x, bar_y, fill_w, 4.0), 2.0),
                    brush.color(&bar_color),
                );
            }
        }
        Precision::DiscreteLevel => {
            let (segments, label) = lang.discrete_info(dev.level);
            draw_text_shadowed(
                rt,
                label,
                &formats.value_discrete,
                rect_xywh(right_x, rect.top + 16.0, right_w, 19.0),
                &val_color,
                shadow_color,
                brush,
            );
            let gap = 3.0;
            let seg_w = (right_w - gap * 3.0) / 4.0;
            for i in 0..4u8 {
                let sx = right_x + (i as f32) * (seg_w + gap);
                let seg_color = if i < segments {
                    &bar_color
                } else {
                    &palette.bar_track
                };
                rt.FillRoundedRectangle(
                    &rounded_rect(rect_xywh(sx, bar_y, seg_w, 4.0), 2.0),
                    brush.color(seg_color),
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
                &formats.value_discrete,
                rect_xywh(right_x, rect.top + 16.0, right_w, 19.0),
                &val_color,
                shadow_color,
                brush,
            );
            rt.FillRoundedRectangle(
                &rounded_rect(rect_xywh(right_x, bar_y, right_w, 4.0), 2.0),
                brush.color(&palette.bar_track),
            );
        }
        Precision::None => {
            draw_text_shadowed(
                rt,
                "--",
                &formats.value_large,
                rect_xywh(right_x, rect.top + 14.0, right_w, 21.0),
                &palette.text_subtle,
                shadow_color,
                brush,
            );
            rt.FillRoundedRectangle(
                &rounded_rect(rect_xywh(right_x, bar_y, right_w, 4.0), 2.0),
                brush.color(&palette.bar_track),
            );
        }
    }

    if !compact {
        let chevron_color = if hovered || expanded {
            &palette.text_secondary
        } else {
            &palette.text_subtle
        };
        draw_text(
            rt,
            if expanded {
                GLYPH_CHEVRON_UP
            } else {
                GLYPH_CHEVRON_DOWN
            },
            &formats.icon_chevron,
            rect_xywh(
                rect.right - OUTER_PADDING - 20.0,
                rect.top + 22.0,
                20.0,
                22.0,
            ),
            chevron_color,
            brush,
        );
    }

    if expanded {
        draw_device_drawer(
            rt,
            formats,
            brush,
            hit_regions,
            rect,
            dev,
            is_stale,
            drawer_alpha,
            palette,
            lang,
            ui,
        );
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_device_drawer(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    row_rect: D2D_RECT_F,
    dev: &DeviceView,
    is_stale: bool,
    drawer_alpha: f32,
    palette: &Palette,
    lang: Lang,
    ui: &UiState,
) {
    let drawer_top = row_rect.top + ROW_COLLAPSED_H + 2.0;
    let drawer_rect = rect_xywh(
        row_rect.left + OUTER_PADDING,
        drawer_top,
        (row_rect.right - row_rect.left) - OUTER_PADDING * 2.0,
        ROW_EXPANDED_EXTRA_H - 12.0,
    );
    let drawer_rr = rounded_rect(drawer_rect, 6.0);
    rt.FillRoundedRectangle(
        &drawer_rr,
        brush.color(&with_alpha(palette.card_drawer_bg, drawer_alpha)),
    );
    rt.DrawRoundedRectangle(
        &drawer_rr,
        brush.color(&with_alpha(palette.card_border, drawer_alpha)),
        1.0,
        None,
    );

    let subtle_color = with_alpha(palette.text_subtle, drawer_alpha);
    let sec_color = with_alpha(palette.text_secondary, drawer_alpha);

    let slide_y = (1.0 - drawer_alpha) * -4.0;
    let pad_x = 12.0;
    let inner_w = (drawer_rect.right - drawer_rect.left) - pad_x * 2.0;
    let col_w = (inner_w - 16.0) * 0.5;
    let col1_x = drawer_rect.left + pad_x;
    let col2_x = col1_x + col_w + 16.0;
    let row1_y = drawer_rect.top + 10.0 + slide_y;
    let row2_y = row1_y + 21.0;

    let proto_label_w = if lang.is_zh() { 34.0 } else { 52.0 };
    draw_text(
        rt,
        lang.label_protocol(),
        &formats.caption,
        rect_xywh(col1_x, row1_y, proto_label_w, 18.0),
        &subtle_color,
        brush,
    );
    draw_text(
        rt,
        dev.provider_id.as_deref().unwrap_or("--"),
        &formats.mono_right,
        rect_xywh(
            col1_x + proto_label_w,
            row1_y,
            (col_w - proto_label_w).max(40.0),
            18.0,
        ),
        &sec_color,
        brush,
    );

    let bus_label_w = if lang.is_zh() { 34.0 } else { 32.0 };
    draw_text(
        rt,
        lang.label_bus(),
        &formats.caption,
        rect_xywh(col2_x, row1_y, bus_label_w, 18.0),
        &subtle_color,
        brush,
    );
    draw_text(
        rt,
        &dev.transport_label,
        &formats.caption_right,
        rect_xywh(
            col2_x + bus_label_w,
            row1_y,
            (col_w - bus_label_w).max(40.0),
            18.0,
        ),
        &sec_color,
        brush,
    );

    let status_label_w = if lang.is_zh() { 34.0 } else { 44.0 };
    draw_text(
        rt,
        lang.label_status(),
        &formats.caption,
        rect_xywh(col1_x, row2_y, status_label_w, 18.0),
        &subtle_color,
        brush,
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
    draw_text(
        rt,
        status_text,
        &formats.caption_right,
        rect_xywh(
            col1_x + status_label_w,
            row2_y,
            (inner_w - status_label_w).max(60.0),
            18.0,
        ),
        &with_alpha(status_color, drawer_alpha),
        brush,
    );

    let btn_y = row2_y + 25.0;
    let has_read_error = dev.last_error.is_some();
    let read_w = if lang.is_zh() { 86.0 } else { 84.0 };
    let read_target = HitTarget::RefreshDevice(dev.id.clone());

    if has_read_error {
        let err_w = if lang.is_zh() { 88.0 } else { 96.0 };
        let err_rect = rect_xywh(drawer_rect.right - pad_x - err_w, btn_y, err_w, 28.0);
        let read_rect = rect_xywh(err_rect.left - 8.0 - read_w, btn_y, read_w, 28.0);
        draw_standard_button(
            rt,
            formats,
            brush,
            hit_regions,
            read_rect,
            Some(GLYPH_REFRESH),
            lang.btn_refresh_device(),
            read_target,
            palette,
            ui,
            drawer_alpha,
        );
        draw_standard_button(
            rt,
            formats,
            brush,
            hit_regions,
            err_rect,
            Some(GLYPH_COPY),
            lang.btn_copy_error(),
            HitTarget::CopyError(dev.id.clone()),
            palette,
            ui,
            drawer_alpha,
        );
    } else {
        let read_rect = rect_xywh(drawer_rect.right - pad_x - read_w, btn_y, read_w, 28.0);
        draw_standard_button(
            rt,
            formats,
            brush,
            hit_regions,
            read_rect,
            Some(GLYPH_REFRESH),
            lang.btn_refresh_device(),
            read_target,
            palette,
            ui,
            drawer_alpha,
        );
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
