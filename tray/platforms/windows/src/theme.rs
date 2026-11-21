//! Windows system light/dark theme detection and Fluent Design color tokens.

use windows::core::w;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

const PERSONALIZE_SUBKEY: windows::core::PCWSTR =
    w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");

/// Current OS appearance flags read from Windows Personalize registry keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTheme {
    /// True when Windows apps are in dark mode (`AppsUseLightTheme == 0`).
    pub app_dark: bool,
    /// True when the Windows taskbar/system shell is in dark mode (`SystemUsesLightTheme == 0`).
    pub taskbar_dark: bool,
}

impl SystemTheme {
    pub fn detect() -> Self {
        let app_light = read_dword(w!("AppsUseLightTheme")).unwrap_or(0);
        let sys_light = read_dword(w!("SystemUsesLightTheme")).unwrap_or(app_light);
        Self {
            app_dark: app_light == 0,
            taskbar_dark: sys_light == 0,
        }
    }
}

fn read_dword(value_name: windows::core::PCWSTR) -> Option<u32> {
    let mut data: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PERSONALIZE_SUBKEY,
            value_name,
            RRF_RT_REG_DWORD,
            None,
            Some((&mut data as *mut u32).cast()),
            Some(&mut size),
        )
    };
    if status.is_ok() {
        Some(data)
    } else {
        None
    }
}

/// Premultiplied-friendly Fluent color palette for Direct2D rendering.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub backdrop_tint: D2D1_COLOR_F,
    pub solid_fallback: D2D1_COLOR_F,
    pub outer_border: D2D1_COLOR_F,
    pub divider: D2D1_COLOR_F,
    pub card_bg: D2D1_COLOR_F,
    pub card_bg_hover: D2D1_COLOR_F,
    pub card_bg_expanded: D2D1_COLOR_F,
    pub card_border: D2D1_COLOR_F,
    pub card_border_hover: D2D1_COLOR_F,
    pub icon_well: D2D1_COLOR_F,
    pub icon_stroke: D2D1_COLOR_F,
    pub pill_bg: D2D1_COLOR_F,
    pub pill_bg_hover: D2D1_COLOR_F,
    pub pill_active: D2D1_COLOR_F,
    pub text_primary: D2D1_COLOR_F,
    pub text_secondary: D2D1_COLOR_F,
    pub text_subtle: D2D1_COLOR_F,
    pub bar_track: D2D1_COLOR_F,
    pub accent_green: D2D1_COLOR_F,
    pub accent_amber: D2D1_COLOR_F,
    pub accent_rose: D2D1_COLOR_F,
    pub accent_sky: D2D1_COLOR_F,
    pub toggle_off: D2D1_COLOR_F,
    pub toggle_thumb: D2D1_COLOR_F,
    pub toast_bg: D2D1_COLOR_F,
    pub toast_text: D2D1_COLOR_F,
}

impl Palette {
    pub fn for_mode(dark: bool) -> Self {
        if dark {
            Self {
                // Translucent tint layered gently over Win11 DWM Transient Acrylic.
                backdrop_tint: rgba(20, 24, 33, 0.56),
                solid_fallback: rgba(22, 26, 35, 0.98),
                outer_border: rgba(255, 255, 255, 0.11),
                divider: rgba(255, 255, 255, 0.08),
                card_bg: rgba(255, 255, 255, 0.045),
                card_bg_hover: rgba(255, 255, 255, 0.082),
                card_bg_expanded: rgba(255, 255, 255, 0.072),
                card_border: rgba(255, 255, 255, 0.065),
                card_border_hover: rgba(255, 255, 255, 0.12),
                icon_well: rgba(255, 255, 255, 0.075),
                icon_stroke: rgba(241, 245, 249, 0.90),
                pill_bg: rgba(255, 255, 255, 0.08),
                pill_bg_hover: rgba(255, 255, 255, 0.14),
                pill_active: rgba(255, 255, 255, 0.18),
                text_primary: rgba(244, 247, 252, 0.96),
                text_secondary: rgba(226, 232, 240, 0.62),
                text_subtle: rgba(226, 232, 240, 0.42),
                bar_track: rgba(255, 255, 255, 0.11),
                accent_green: rgba(52, 211, 153, 1.0),
                accent_amber: rgba(251, 191, 36, 1.0),
                accent_rose: rgba(248, 113, 113, 1.0),
                accent_sky: rgba(56, 189, 248, 1.0),
                toggle_off: rgba(255, 255, 255, 0.18),
                toggle_thumb: rgba(255, 255, 255, 0.98),
                toast_bg: rgba(16, 185, 129, 0.95),
                toast_text: rgba(255, 255, 255, 1.0),
            }
        } else {
            Self {
                backdrop_tint: rgba(250, 252, 255, 0.66),
                solid_fallback: rgba(248, 250, 253, 0.98),
                outer_border: rgba(15, 23, 42, 0.10),
                divider: rgba(15, 23, 42, 0.08),
                card_bg: rgba(255, 255, 255, 0.68),
                card_bg_hover: rgba(255, 255, 255, 0.92),
                card_bg_expanded: rgba(255, 255, 255, 0.95),
                card_border: rgba(15, 23, 42, 0.065),
                card_border_hover: rgba(15, 23, 42, 0.12),
                icon_well: rgba(15, 23, 42, 0.055),
                icon_stroke: rgba(15, 23, 42, 0.84),
                pill_bg: rgba(15, 23, 42, 0.065),
                pill_bg_hover: rgba(15, 23, 42, 0.11),
                pill_active: rgba(255, 255, 255, 0.98),
                text_primary: rgba(15, 23, 42, 0.95),
                text_secondary: rgba(51, 65, 85, 0.70),
                text_subtle: rgba(51, 65, 85, 0.48),
                bar_track: rgba(15, 23, 42, 0.10),
                accent_green: rgba(16, 185, 129, 1.0),
                accent_amber: rgba(217, 119, 6, 1.0),
                accent_rose: rgba(225, 29, 72, 1.0),
                accent_sky: rgba(2, 132, 199, 1.0),
                toggle_off: rgba(15, 23, 42, 0.20),
                toggle_thumb: rgba(255, 255, 255, 1.0),
                toast_bg: rgba(16, 185, 129, 0.96),
                toast_text: rgba(255, 255, 255, 1.0),
            }
        }
    }
}

pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: (r as f32) / 255.0,
        g: (g as f32) / 255.0,
        b: (b as f32) / 255.0,
        a,
    }
}
