//! Settings view cards, WinUI 3 ComboBox dropdowns, ToggleSwitches, and updater card.

use super::anim::{
    CommunityLinkKind, HitRegion, HitTarget, OpenDropdown, UiState, UpdateUiStatus, CARD_RADIUS,
    CONTROL_RADIUS, HEADER_HEIGHT, OUTER_PADDING, SETTINGS_ACTION_BUTTON_H,
    SETTINGS_ACTION_BUTTON_W_EN, SETTINGS_ACTION_BUTTON_W_ZH, SETTINGS_CARD_GAP, SETTINGS_CARD_H,
    SETTINGS_ICON_BUTTON_GAP, SETTINGS_ICON_BUTTON_SIZE, SETTINGS_SUBROW_H,
};
use super::widgets::{
    draw_community_icon, draw_community_icon_button, draw_elevated_card, draw_standard_button,
    draw_subtle_icon_button, draw_text, draw_win11_toggle_switch, rect_xywh, rounded_rect,
    PaintBrush, TextFormats, GLYPH_BACK, GLYPH_CHEVRON_DOWN, GLYPH_CHEVRON_UP, GLYPH_CLOCK,
    GLYPH_COLOR_THEME, GLYPH_INFO, GLYPH_LANGUAGE, GLYPH_PEOPLE, GLYPH_PIN, GLYPH_POWER,
    GLYPH_RINGER, GLYPH_RINGER_SILENT,
};
use crate::i18n::Lang;
use crate::theme::{with_alpha, Palette, SystemTheme};
use std::time::Instant;
use windows::Win32::Graphics::Direct2D::Common::{D2D_POINT_2F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Factory, ID2D1HwndRenderTarget, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
};
use wireless_battery_tray_core::{LanguageMode, ThemeMode, TraySnapshot};

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_settings_view(
    d2d: &ID2D1Factory,
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
    system_theme: SystemTheme,
    snapshot: &TraySnapshot,
    ui: &mut UiState,
    now: Instant,
) {
    // --- Win11 Settings Header (52 DIPs) ---
    let back_rect = rect_xywh(12.0, 10.0, 32.0, 32.0);
    draw_subtle_icon_button(
        rt,
        formats,
        brush,
        hit_regions,
        back_rect,
        GLYPH_BACK,
        &HitTarget::BackToMain,
        false,
        palette,
        ui,
    );

    draw_text(
        rt,
        lang.settings_title(),
        &formats.title,
        rect_xywh(50.0, 10.0, width - 66.0, 32.0),
        &palette.text_primary,
        brush,
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

    // --- Flat Independent WinUI 3 SettingsCards (4px gap) ---
    let subrow_extra = if snapshot.config.low_battery_notify {
        SETTINGS_SUBROW_H
    } else {
        0.0
    };
    *viewport_height = (height - HEADER_HEIGHT).max(1.0);
    *content_height =
        OUTER_PADDING * 2.0 + SETTINGS_CARD_H * 10.0 + SETTINGS_CARD_GAP * 9.0 + subrow_extra;
    let max_scroll = (*content_height - *viewport_height).max(0.0);
    ui.scroll_offset = ui.scroll_offset.clamp(0.0, max_scroll);

    rt.PushAxisAlignedClip(
        &D2D_RECT_F {
            left: 0.0,
            top: HEADER_HEIGHT,
            right: width,
            bottom: height,
        },
        D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
    );

    let pad_x = OUTER_PADDING;
    let inner_w = width - pad_x * 2.0;
    let mut y = HEADER_HEIGHT + OUTER_PADDING - ui.scroll_offset;
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
        formats,
        brush,
        theme_card,
        GLYPH_COLOR_THEME,
        lang.setting_theme_title(),
        lang.setting_theme_desc(snapshot.config.theme_mode, system_theme.app_dark),
        combo_w + 22.0,
        palette,
        false,
        false,
    );
    draw_combobox_button(
        rt,
        formats,
        brush,
        hit_regions,
        theme_combo_rect,
        lang.theme_mode_label(snapshot.config.theme_mode),
        OpenDropdown::ThemeMode,
        palette,
        ui,
    );
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
        formats,
        brush,
        lang_card,
        GLYPH_LANGUAGE,
        lang.setting_language_title(),
        lang.setting_language_desc(snapshot.config.language),
        combo_w + 22.0,
        palette,
        false,
        false,
    );
    draw_combobox_button(
        rt,
        formats,
        brush,
        hit_regions,
        lang_combo_rect,
        lang.language_mode_label(snapshot.config.language),
        OpenDropdown::Language,
        palette,
        ui,
    );
    y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

    // Card 3: Poll Interval (ComboBox)
    let poll_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
    let poll_combo_rect = rect_xywh(
        poll_card.right - 14.0 - combo_w,
        poll_card.top + (SETTINGS_CARD_H - combo_h) * 0.5,
        combo_w,
        combo_h,
    );
    draw_settings_card_base(
        rt,
        formats,
        brush,
        poll_card,
        GLYPH_CLOCK,
        lang.setting_poll_title(),
        lang.setting_poll_desc(),
        combo_w + 22.0,
        palette,
        false,
        false,
    );
    let poll_label = lang.poll_interval_label(snapshot.config.poll_interval_secs);
    draw_combobox_button(
        rt,
        formats,
        brush,
        hit_regions,
        poll_combo_rect,
        &poll_label,
        OpenDropdown::PollInterval,
        palette,
        ui,
    );
    y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

    // Card 4: Low Battery Alert + nested Silent Notifications sub-option
    let notify_card_h = draw_low_battery_card(
        rt,
        formats,
        brush,
        hit_regions,
        pad_x,
        y,
        inner_w,
        palette,
        lang,
        snapshot,
        ui,
        now,
    );
    y += notify_card_h + SETTINGS_CARD_GAP;

    // Card 5: Launch at Startup (ToggleSwitch)
    let auto_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
    let auto_target = HitTarget::ToggleAutostart;
    let auto_hovered = ui.open_dropdown.is_none() && ui.hovered.as_ref() == Some(&auto_target);
    let auto_pressed = ui.open_dropdown.is_none() && ui.pressed.as_ref() == Some(&auto_target);
    draw_settings_card_base(
        rt,
        formats,
        brush,
        auto_card,
        GLYPH_POWER,
        lang.setting_autostart_title(),
        lang.setting_autostart_desc(),
        64.0,
        palette,
        auto_hovered,
        auto_pressed,
    );
    draw_win11_toggle_switch(
        rt,
        brush,
        auto_card.right - 14.0 - 40.0,
        auto_card.top + (SETTINGS_CARD_H - 20.0) * 0.5,
        ui.toggle_autostart_anim.value(now),
        auto_hovered,
        auto_pressed,
        palette,
    );
    hit_regions.push(HitRegion {
        rect: auto_card,
        target: auto_target,
    });
    y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

    // Card 6: Show Pin Button (ToggleSwitch)
    let pin_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
    let pin_target = HitTarget::ToggleShowPinButton;
    let pin_hovered = ui.open_dropdown.is_none() && ui.hovered.as_ref() == Some(&pin_target);
    let pin_pressed = ui.open_dropdown.is_none() && ui.pressed.as_ref() == Some(&pin_target);
    draw_settings_card_base(
        rt,
        formats,
        brush,
        pin_card,
        GLYPH_PIN,
        lang.setting_show_pin_button_title(),
        lang.setting_show_pin_button_desc(),
        64.0,
        palette,
        pin_hovered,
        pin_pressed,
    );
    draw_win11_toggle_switch(
        rt,
        brush,
        pin_card.right - 14.0 - 40.0,
        pin_card.top + (SETTINGS_CARD_H - 20.0) * 0.5,
        ui.toggle_show_pin_button_anim.value(now),
        pin_hovered,
        pin_pressed,
        palette,
    );
    hit_regions.push(HitRegion {
        rect: pin_card,
        target: pin_target,
    });
    y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

    let action_btn_w = if lang.is_zh() {
        SETTINGS_ACTION_BUTTON_W_ZH
    } else {
        SETTINGS_ACTION_BUTTON_W_EN
    };
    let action_btn_h = SETTINGS_ACTION_BUTTON_H;

    // Card 7: Version & Check for Updates
    let ver_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
    let ver_desc = match &ui.update_status {
        UpdateUiStatus::Idle => lang.setting_version_desc(&snapshot.app_version),
        UpdateUiStatus::Checking => lang.setting_version_checking().to_owned(),
        UpdateUiStatus::UpToDate => lang.setting_version_up_to_date(&snapshot.app_version),
        UpdateUiStatus::Available { latest_version, .. } => {
            lang.setting_version_available(latest_version)
        }
        UpdateUiStatus::Failed => lang.setting_version_failed().to_owned(),
    };
    let btn_label = match &ui.update_status {
        UpdateUiStatus::Checking => lang.btn_checking_updates(),
        UpdateUiStatus::Available { .. } => lang.btn_download_update(),
        UpdateUiStatus::Idle | UpdateUiStatus::UpToDate | UpdateUiStatus::Failed => {
            lang.btn_check_updates()
        }
    };
    draw_settings_card_base(
        rt,
        formats,
        brush,
        ver_card,
        GLYPH_INFO,
        lang.setting_version_title(),
        &ver_desc,
        action_btn_w + 22.0,
        palette,
        false,
        false,
    );
    let btn_rect = rect_xywh(
        ver_card.right - 14.0 - action_btn_w,
        ver_card.top + (SETTINGS_CARD_H - action_btn_h) * 0.5,
        action_btn_w,
        action_btn_h,
    );
    let btn_alpha = if matches!(ui.update_status, UpdateUiStatus::Checking) {
        0.55
    } else {
        1.0
    };
    draw_standard_button(
        rt,
        formats,
        brush,
        hit_regions,
        btn_rect,
        None,
        btn_label,
        HitTarget::CheckUpdates,
        palette,
        ui,
        btn_alpha,
    );
    y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

    // Card 8: Source Code
    let source_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
    draw_settings_card_base(
        rt,
        formats,
        brush,
        source_card,
        "",
        lang.setting_source_title(),
        lang.setting_source_desc(),
        action_btn_w + 22.0,
        palette,
        false,
        false,
    );
    draw_community_icon(
        d2d,
        rt,
        formats,
        brush,
        CommunityLinkKind::SourceRepo,
        rect_xywh(source_card.left + 17.0, source_card.top + 20.0, 16.0, 16.0),
        &palette.icon_stroke,
    );
    let source_btn_rect = rect_xywh(
        source_card.right - 14.0 - action_btn_w,
        source_card.top + (SETTINGS_CARD_H - action_btn_h) * 0.5,
        action_btn_w,
        action_btn_h,
    );
    draw_standard_button(
        rt,
        formats,
        brush,
        hit_regions,
        source_btn_rect,
        None,
        lang.open_repository_label(),
        HitTarget::OpenCommunityLink(CommunityLinkKind::SourceRepo),
        palette,
        ui,
        1.0,
    );
    y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

    // Card 9: Feedback
    let feedback_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
    draw_settings_card_base(
        rt,
        formats,
        brush,
        feedback_card,
        "",
        lang.setting_feedback_title(),
        lang.setting_feedback_desc(),
        action_btn_w + 22.0,
        palette,
        false,
        false,
    );
    draw_community_icon(
        d2d,
        rt,
        formats,
        brush,
        CommunityLinkKind::BugReport,
        rect_xywh(
            feedback_card.left + 17.0,
            feedback_card.top + 20.0,
            16.0,
            16.0,
        ),
        &palette.icon_stroke,
    );
    let feedback_btn_rect = rect_xywh(
        feedback_card.right - 14.0 - action_btn_w,
        feedback_card.top + (SETTINGS_CARD_H - action_btn_h) * 0.5,
        action_btn_w,
        action_btn_h,
    );
    draw_standard_button(
        rt,
        formats,
        brush,
        hit_regions,
        feedback_btn_rect,
        None,
        lang.submit_feedback_label(),
        HitTarget::OpenCommunityLink(CommunityLinkKind::BugReport),
        palette,
        ui,
        1.0,
    );
    y += SETTINGS_CARD_H + SETTINGS_CARD_GAP;

    // Card 10: Community (QQ & Discord)
    let community_card = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
    let icon_btn_size = SETTINGS_ICON_BUTTON_SIZE;
    let icon_gap = SETTINGS_ICON_BUTTON_GAP;
    let community_buttons_w = icon_btn_size * 2.0 + icon_gap;
    draw_settings_card_base(
        rt,
        formats,
        brush,
        community_card,
        GLYPH_PEOPLE,
        lang.setting_community_title(),
        lang.setting_community_desc(),
        community_buttons_w + 22.0,
        palette,
        false,
        false,
    );
    let community_btn_y = community_card.top + (SETTINGS_CARD_H - icon_btn_size) * 0.5;
    let discord_rect = rect_xywh(
        community_card.right - 14.0 - icon_btn_size,
        community_btn_y,
        icon_btn_size,
        icon_btn_size,
    );
    let qq_rect = rect_xywh(
        discord_rect.left - icon_gap - icon_btn_size,
        community_btn_y,
        icon_btn_size,
        icon_btn_size,
    );
    draw_community_icon_button(
        d2d,
        rt,
        formats,
        brush,
        hit_regions,
        qq_rect,
        CommunityLinkKind::Qq,
        palette,
        ui,
    );
    draw_community_icon_button(
        d2d,
        rt,
        formats,
        brush,
        hit_regions,
        discord_rect,
        CommunityLinkKind::Discord,
        palette,
        ui,
    );

    if max_scroll > 0.5 {
        let track_top = HEADER_HEIGHT + 10.0;
        let track_h = (height - HEADER_HEIGHT - 20.0).max(20.0);
        let thumb_h = ((*viewport_height / *content_height) * track_h).clamp(28.0, track_h);
        let scroll_ratio = ui.scroll_offset / max_scroll;
        let thumb_y = track_top + scroll_ratio * (track_h - thumb_h);
        rt.FillRoundedRectangle(
            &rounded_rect(rect_xywh(width - 6.0, thumb_y, 3.0, thumb_h), 1.5),
            brush.color(&palette.scrollbar_thumb),
        );
    }

    rt.PopAxisAlignedClip();

    // --- WinUI 3 ComboBox Dropdown Flyout Overlay (if open) ---
    if let Some(open) = ui.open_dropdown {
        hit_regions.push(HitRegion {
            rect: rect_xywh(0.0, 0.0, width, height),
            target: HitTarget::CloseDropdown,
        });
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
                draw_dropdown_flyout(
                    rt,
                    formats,
                    brush,
                    hit_regions,
                    theme_combo_rect,
                    height,
                    &options,
                    anim,
                    palette,
                    ui,
                );
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
                draw_dropdown_flyout(
                    rt,
                    formats,
                    brush,
                    hit_regions,
                    lang_combo_rect,
                    height,
                    &options,
                    anim,
                    palette,
                    ui,
                );
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
                draw_dropdown_flyout(
                    rt,
                    formats,
                    brush,
                    hit_regions,
                    poll_combo_rect,
                    height,
                    &options,
                    anim,
                    palette,
                    ui,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_low_battery_card(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    pad_x: f32,
    y: f32,
    inner_w: f32,
    palette: &Palette,
    lang: Lang,
    snapshot: &TraySnapshot,
    ui: &UiState,
    now: Instant,
) -> f32 {
    let show_subrow = snapshot.config.low_battery_notify;
    let notify_card_h = if show_subrow {
        SETTINGS_CARD_H + SETTINGS_SUBROW_H
    } else {
        SETTINGS_CARD_H
    };
    let notify_card = rect_xywh(pad_x, y, inner_w, notify_card_h);
    let notify_header_rect = rect_xywh(pad_x, y, inner_w, SETTINGS_CARD_H);
    let notify_target = HitTarget::ToggleLowBatteryNotify;
    let notify_hovered = ui.open_dropdown.is_none() && ui.hovered.as_ref() == Some(&notify_target);
    let notify_pressed = ui.open_dropdown.is_none() && ui.pressed.as_ref() == Some(&notify_target);

    let card_bg = if !show_subrow && notify_pressed {
        palette.card_bg_pressed
    } else if !show_subrow && notify_hovered {
        palette.card_bg_hover
    } else {
        palette.card_bg
    };
    draw_elevated_card(
        rt,
        brush,
        notify_card,
        CARD_RADIUS,
        card_bg,
        palette.card_border,
        palette,
    );

    if show_subrow && (notify_hovered || notify_pressed) {
        let row_fill = if notify_pressed {
            &palette.subtle_pressed
        } else {
            &palette.subtle_hover
        };
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
            &rounded_rect(
                D2D_RECT_F {
                    left: notify_card.left + 1.0,
                    top: notify_card.top + 1.0,
                    right: notify_card.right - 1.0,
                    bottom: notify_card.bottom - 1.0,
                },
                CARD_RADIUS - 1.0,
            ),
            brush.color(row_fill),
        );
        rt.PopAxisAlignedClip();
    }

    draw_settings_row_content(
        rt,
        formats,
        brush,
        notify_header_rect,
        GLYPH_RINGER,
        lang.setting_low_battery_title(),
        lang.setting_low_battery_desc(
            snapshot.config.low_battery_notify,
            snapshot.config.low_battery_silent,
        ),
        64.0,
        palette,
    );
    draw_win11_toggle_switch(
        rt,
        brush,
        notify_header_rect.right - 14.0 - 40.0,
        notify_header_rect.top + (SETTINGS_CARD_H - 20.0) * 0.5,
        ui.toggle_notify_anim.value(now),
        notify_hovered,
        notify_pressed,
        palette,
    );
    hit_regions.push(HitRegion {
        rect: notify_header_rect,
        target: notify_target,
    });

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

        rt.PushAxisAlignedClip(&sub_rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        let sub_bg_color = if silent_pressed {
            palette.card_bg_pressed
        } else if silent_hovered {
            palette.card_bg_hover
        } else {
            palette.card_drawer_bg
        };
        rt.FillRoundedRectangle(
            &rounded_rect(
                D2D_RECT_F {
                    left: notify_card.left + 0.5,
                    top: notify_card.top + 0.5,
                    right: notify_card.right - 0.5,
                    bottom: notify_card.bottom - 0.5,
                },
                CARD_RADIUS - 0.5,
            ),
            brush.color(&sub_bg_color),
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
            brush.color(&palette.divider),
            1.0,
            None,
        );

        draw_text(
            rt,
            GLYPH_RINGER_SILENT,
            &formats.icon_small,
            rect_xywh(notify_card.left + 26.0, sub_top, 18.0, SETTINGS_SUBROW_H),
            &palette.text_secondary,
            brush,
        );
        let sub_text_x = notify_card.left + 48.0;
        let sub_text_w = ((notify_card.right - notify_card.left) - 48.0 - 64.0).max(60.0);
        draw_text(
            rt,
            lang.setting_silent_title(),
            &formats.body,
            rect_xywh(sub_text_x, sub_top + 7.0, sub_text_w, 18.0),
            &palette.text_primary,
            brush,
        );
        draw_text(
            rt,
            lang.setting_silent_desc(),
            &formats.caption,
            rect_xywh(sub_text_x, sub_top + 25.0, sub_text_w, 16.0),
            &palette.text_secondary,
            brush,
        );
        draw_win11_toggle_switch(
            rt,
            brush,
            notify_card.right - 14.0 - 40.0,
            sub_top + (SETTINGS_SUBROW_H - 20.0) * 0.5,
            ui.toggle_silent_anim.value(now),
            silent_hovered,
            silent_pressed,
            palette,
        );
        hit_regions.push(HitRegion {
            rect: sub_rect,
            target: silent_target,
        });
    }
    notify_card_h
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_settings_card_base(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    rect: D2D_RECT_F,
    glyph: &str,
    title: &str,
    desc: &str,
    right_reserved_w: f32,
    palette: &Palette,
    hovered: bool,
    pressed: bool,
) {
    let bg = if pressed {
        palette.card_bg_pressed
    } else if hovered {
        palette.card_bg_hover
    } else {
        palette.card_bg
    };
    draw_elevated_card(
        rt,
        brush,
        rect,
        CARD_RADIUS,
        bg,
        palette.card_border,
        palette,
    );
    draw_settings_row_content(
        rt,
        formats,
        brush,
        rect,
        glyph,
        title,
        desc,
        right_reserved_w,
        palette,
    );
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_settings_row_content(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    rect: D2D_RECT_F,
    glyph: &str,
    title: &str,
    desc: &str,
    right_reserved_w: f32,
    palette: &Palette,
) {
    let card_h = rect.bottom - rect.top;
    draw_text(
        rt,
        glyph,
        &formats.icon_setting,
        rect_xywh(rect.left + 14.0, rect.top, 22.0, card_h),
        &palette.icon_stroke,
        brush,
    );

    let text_x = rect.left + 46.0;
    let text_w = ((rect.right - rect.left) - 46.0 - right_reserved_w).max(60.0);
    draw_text(
        rt,
        title,
        &formats.body,
        rect_xywh(text_x, rect.top + 9.0, text_w, 19.0),
        &palette.text_primary,
        brush,
    );
    draw_text(
        rt,
        desc,
        &formats.caption,
        rect_xywh(text_x, rect.top + 29.0, text_w, 18.0),
        &palette.text_secondary,
        brush,
    );
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_combobox_button(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    rect: D2D_RECT_F,
    label: &str,
    dropdown: OpenDropdown,
    palette: &Palette,
    ui: &UiState,
) {
    let target = HitTarget::ToggleDropdown(dropdown);
    let is_open = ui.open_dropdown == Some(dropdown);
    let hovered = ui.open_dropdown.is_none() && ui.hovered.as_ref() == Some(&target);
    let pressed = ui.open_dropdown.is_none() && ui.pressed.as_ref() == Some(&target);
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
    draw_elevated_card(rt, brush, rect, CONTROL_RADIUS, bg, border, palette);

    let h = rect.bottom - rect.top;
    draw_text(
        rt,
        label,
        &formats.body,
        rect_xywh(
            rect.left + 11.0,
            rect.top,
            (rect.right - rect.left) - 32.0,
            h,
        ),
        &palette.text_primary,
        brush,
    );
    draw_text(
        rt,
        if is_open {
            GLYPH_CHEVRON_UP
        } else {
            GLYPH_CHEVRON_DOWN
        },
        &formats.icon_chevron,
        rect_xywh(rect.right - 24.0, rect.top, 16.0, h),
        &palette.text_secondary,
        brush,
    );
    hit_regions.push(HitRegion { rect, target });
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_dropdown_flyout<S: AsRef<str>>(
    rt: &ID2D1HwndRenderTarget,
    formats: &TextFormats,
    brush: &PaintBrush,
    hit_regions: &mut Vec<HitRegion>,
    anchor: D2D_RECT_F,
    panel_height: f32,
    options: &[(HitTarget, S, bool)],
    anim: f32,
    palette: &Palette,
    ui: &UiState,
) {
    let menu_w = 132.0_f32.max(anchor.right - anchor.left);
    let item_h = 32.0;
    let pad = 4.0;
    let menu_h = pad * 2.0 + (options.len() as f32) * item_h;
    let slide_y = (1.0 - anim) * -6.0;
    let preferred_top = anchor.bottom + 4.0;
    let max_top = (panel_height - menu_h - 10.0).max(HEADER_HEIGHT + 6.0);
    let menu_top = preferred_top.min(max_top) + slide_y;
    let menu_rect = rect_xywh(anchor.right - menu_w, menu_top, menu_w, menu_h);

    rt.FillRoundedRectangle(
        &rounded_rect(
            D2D_RECT_F {
                left: menu_rect.left - 3.0,
                top: menu_rect.top + 3.0,
                right: menu_rect.right + 3.0,
                bottom: menu_rect.bottom + 8.0,
            },
            CARD_RADIUS + 2.0,
        ),
        brush.color(&with_alpha(palette.flyout_shadow, anim * 0.45)),
    );
    rt.FillRoundedRectangle(
        &rounded_rect(
            D2D_RECT_F {
                left: menu_rect.left - 1.0,
                top: menu_rect.top + 1.0,
                right: menu_rect.right + 1.0,
                bottom: menu_rect.bottom + 3.0,
            },
            CARD_RADIUS + 1.0,
        ),
        brush.color(&with_alpha(palette.flyout_shadow, anim * 0.85)),
    );

    let rr = rounded_rect(
        D2D_RECT_F {
            left: menu_rect.left + 0.5,
            top: menu_rect.top + 0.5,
            right: menu_rect.right - 0.5,
            bottom: menu_rect.bottom - 0.5,
        },
        CARD_RADIUS,
    );
    rt.FillRoundedRectangle(&rr, brush.color(&with_alpha(palette.flyout_bg, anim)));
    rt.DrawRoundedRectangle(
        &rr,
        brush.color(&with_alpha(palette.flyout_border, anim)),
        1.0,
        None,
    );

    let text_color = with_alpha(palette.text_primary, anim);
    let accent_color = with_alpha(palette.accent_fill, anim);

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
            rt.FillRoundedRectangle(
                &rounded_rect(item_rect, CONTROL_RADIUS),
                brush.color(&with_alpha(fill, anim)),
            );
        }

        if *selected {
            rt.FillRoundedRectangle(
                &rounded_rect(
                    rect_xywh(
                        item_rect.left + 1.0,
                        item_rect.top + (item_h - 16.0) * 0.5,
                        3.0,
                        16.0,
                    ),
                    1.5,
                ),
                brush.color(&accent_color),
            );
        }

        draw_text(
            rt,
            label.as_ref(),
            &formats.body,
            rect_xywh(
                item_rect.left + 12.0,
                item_rect.top,
                (item_rect.right - item_rect.left) - 18.0,
                item_h,
            ),
            &text_color,
            brush,
        );

        hit_regions.push(HitRegion {
            rect: item_rect,
            target: target.clone(),
        });
    }
}
