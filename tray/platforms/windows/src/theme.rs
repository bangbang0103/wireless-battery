//! Windows 11 Fluent 2 system theme detection, accent color derivation,
//! and WinUI 3 color tokens for Direct2D rendering.

use windows::core::{w, PCWSTR};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use wireless_battery_tray_core::ThemeMode;

const PERSONALIZE_SUBKEY: PCWSTR =
    w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
const DWM_SUBKEY: PCWSTR = w!(r"Software\Microsoft\Windows\DWM");

/// Default Windows 11 Fluent accent blue (`#0078D4`) when registry accent is unavailable.
const DEFAULT_ACCENT_RGB: (u8, u8, u8) = (0, 120, 212);

/// Current OS appearance flags read from Windows Personalize & DWM registry keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTheme {
    /// True when Windows apps are in dark mode (`AppsUseLightTheme == 0`).
    pub app_dark: bool,
    /// True when the Windows taskbar/system shell is in dark mode (`SystemUsesLightTheme == 0`).
    pub taskbar_dark: bool,
    /// True when Windows Transparency Effects are enabled (`EnableTransparency != 0`).
    pub transparency_enabled: bool,
    /// Windows system accent color `(R, G, B)` from `HKCU\Software\Microsoft\Windows\DWM`.
    pub accent_rgb: (u8, u8, u8),
}

impl SystemTheme {
    pub fn detect() -> Self {
        let app_light = read_dword(PERSONALIZE_SUBKEY, w!("AppsUseLightTheme")).unwrap_or(0);
        let sys_light =
            read_dword(PERSONALIZE_SUBKEY, w!("SystemUsesLightTheme")).unwrap_or(app_light);
        let transparency = read_dword(PERSONALIZE_SUBKEY, w!("EnableTransparency")).unwrap_or(1);
        let accent_rgb = detect_accent_rgb();
        Self {
            app_dark: app_light == 0,
            taskbar_dark: sys_light == 0,
            transparency_enabled: transparency != 0,
            accent_rgb,
        }
    }

    /// Resolve the effective dark-mode flag for a user-selected `ThemeMode`.
    #[must_use]
    pub const fn resolve_dark(self, mode: ThemeMode) -> bool {
        match mode {
            ThemeMode::System => self.app_dark,
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
        }
    }
}

fn detect_accent_rgb() -> (u8, u8, u8) {
    // `AccentColor` is stored in ABGR byte order (0xAABBGGRR).
    if let Some(abgr) = read_dword(DWM_SUBKEY, w!("AccentColor")) {
        let r = (abgr & 0xFF) as u8;
        let g = ((abgr >> 8) & 0xFF) as u8;
        let b = ((abgr >> 16) & 0xFF) as u8;
        if r != 0 || g != 0 || b != 0 {
            return (r, g, b);
        }
    }
    // Fallback: `ColorizationColor` is stored in ARGB byte order (0xAARRGGBB).
    if let Some(argb) = read_dword(DWM_SUBKEY, w!("ColorizationColor")) {
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;
        if r != 0 || g != 0 || b != 0 {
            return (r, g, b);
        }
    }
    DEFAULT_ACCENT_RGB
}

fn read_dword(subkey: PCWSTR, value_name: PCWSTR) -> Option<u32> {
    let mut data: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey,
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

/// Authentic Windows 11 Fluent 2 / WinUI 3 color palette for Direct2D rendering.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    // Window & layer surfaces
    pub backdrop_tint: D2D1_COLOR_F,
    pub solid_fallback: D2D1_COLOR_F,
    pub footer_bg: D2D1_COLOR_F,
    pub outer_border: D2D1_COLOR_F,
    pub divider: D2D1_COLOR_F,
    // Cards / WinUI 3 Expander surfaces
    pub card_bg: D2D1_COLOR_F,
    pub card_bg_hover: D2D1_COLOR_F,
    pub card_bg_pressed: D2D1_COLOR_F,
    pub card_drawer_bg: D2D1_COLOR_F,
    pub card_border: D2D1_COLOR_F,
    pub elevation_edge: D2D1_COLOR_F,
    // Standard & subtle controls
    pub icon_well: D2D1_COLOR_F,
    pub icon_stroke: D2D1_COLOR_F,
    pub control_bg: D2D1_COLOR_F,
    pub control_bg_hover: D2D1_COLOR_F,
    pub control_bg_pressed: D2D1_COLOR_F,
    pub control_border: D2D1_COLOR_F,
    pub subtle_hover: D2D1_COLOR_F,
    pub subtle_pressed: D2D1_COLOR_F,
    // WinUI 3 ComboBox dropdown flyout
    pub flyout_bg: D2D1_COLOR_F,
    pub flyout_border: D2D1_COLOR_F,
    pub flyout_shadow: D2D1_COLOR_F,
    // Compact pinned transparent-mode text shadow
    pub compact_text_shadow: D2D1_COLOR_F,
    // Typography
    pub text_primary: D2D1_COLOR_F,
    pub text_secondary: D2D1_COLOR_F,
    pub text_subtle: D2D1_COLOR_F,
    // System accent tokens (derived from Windows AccentColor)
    pub accent_fill: D2D1_COLOR_F,
    pub accent_fill_hover: D2D1_COLOR_F,
    pub accent_text: D2D1_COLOR_F,
    pub accent_subtle_bg: D2D1_COLOR_F,
    pub accent_subtle_border: D2D1_COLOR_F,
    pub text_on_accent: D2D1_COLOR_F,
    // WinUI 3 semantic status colors
    pub bar_track: D2D1_COLOR_F,
    pub status_good: D2D1_COLOR_F,
    pub status_good_muted: D2D1_COLOR_F,
    pub status_caution: D2D1_COLOR_F,
    pub status_critical: D2D1_COLOR_F,
    // WinUI 3 ToggleSwitch (OFF state)
    pub toggle_off_bg: D2D1_COLOR_F,
    pub toggle_off_bg_hover: D2D1_COLOR_F,
    pub toggle_off_border: D2D1_COLOR_F,
    pub toggle_off_thumb: D2D1_COLOR_F,
    // Floating feedback toast (InfoBadge / TeachingTip surface)
    pub toast_bg: D2D1_COLOR_F,
    pub toast_border: D2D1_COLOR_F,
    pub toast_text: D2D1_COLOR_F,
    // Scrollbar thumb
    pub scrollbar_thumb: D2D1_COLOR_F,
}

impl Palette {
    pub fn for_theme(dark: bool, accent_rgb: (u8, u8, u8)) -> Self {
        let (
            accent_fill,
            accent_fill_hover,
            accent_text,
            accent_subtle_bg,
            accent_subtle_border,
            text_on_accent,
        ) = derive_accent_tokens(dark, accent_rgb);

        if dark {
            Self {
                dark: true,
                // Neutral Windows 11 dark charcoal (#202020), letting DWM Acrylic texture breathe.
                backdrop_tint: rgba(32, 32, 32, 0.30),
                solid_fallback: rgba(32, 32, 32, 1.0),
                footer_bg: rgba(0, 0, 0, 0.24),
                outer_border: rgba(255, 255, 255, 0.095),
                divider: rgba(255, 255, 255, 0.084),
                // WinUI 3 CardBackgroundFillColorDefault / ControlFillColor*
                card_bg: rgba(255, 255, 255, 0.051),
                card_bg_hover: rgba(255, 255, 255, 0.084),
                card_bg_pressed: rgba(255, 255, 255, 0.033),
                card_drawer_bg: rgba(0, 0, 0, 0.14),
                card_border: rgba(255, 255, 255, 0.070),
                // Top specular highlight in Dark Mode (ControlElevationBorder)
                elevation_edge: rgba(255, 255, 255, 0.095),
                icon_well: rgba(255, 255, 255, 0.058),
                icon_stroke: rgba(255, 255, 255, 0.92),
                control_bg: rgba(255, 255, 255, 0.061),
                control_bg_hover: rgba(255, 255, 255, 0.086),
                control_bg_pressed: rgba(255, 255, 255, 0.035),
                control_border: rgba(255, 255, 255, 0.075),
                subtle_hover: rgba(255, 255, 255, 0.061),
                subtle_pressed: rgba(255, 255, 255, 0.042),
                flyout_bg: rgba(44, 44, 44, 0.98),
                flyout_border: rgba(255, 255, 255, 0.115),
                flyout_shadow: rgba(0, 0, 0, 0.34),
                compact_text_shadow: rgba(0, 0, 0, 0.52),
                text_primary: rgba(255, 255, 255, 1.0),
                text_secondary: rgba(255, 255, 255, 0.786),
                text_subtle: rgba(255, 255, 255, 0.544),
                accent_fill,
                accent_fill_hover,
                accent_text,
                accent_subtle_bg,
                accent_subtle_border,
                text_on_accent,
                bar_track: rgba(255, 255, 255, 0.15),
                // Official WinUI 3 Dark semantic status colors
                status_good: rgba(108, 203, 95, 1.0),
                status_good_muted: rgba(108, 203, 95, 0.48),
                status_caution: rgba(252, 225, 0, 1.0),
                status_critical: rgba(255, 153, 164, 1.0),
                toggle_off_bg: rgba(0, 0, 0, 0.10),
                toggle_off_bg_hover: rgba(255, 255, 255, 0.042),
                toggle_off_border: rgba(255, 255, 255, 0.605),
                toggle_off_thumb: rgba(255, 255, 255, 0.786),
                toast_bg: rgba(44, 44, 44, 0.96),
                toast_border: rgba(255, 255, 255, 0.12),
                toast_text: rgba(255, 255, 255, 0.96),
                scrollbar_thumb: rgba(255, 255, 255, 0.34),
            }
        } else {
            Self {
                dark: false,
                // Crisp neutral Windows 11 Light Acrylic (#F9F9F9 / #F3F3F3), zero blue cast.
                backdrop_tint: rgba(249, 249, 249, 0.48),
                solid_fallback: rgba(243, 243, 243, 1.0),
                footer_bg: rgba(0, 0, 0, 0.036),
                outer_border: rgba(0, 0, 0, 0.085),
                divider: rgba(0, 0, 0, 0.075),
                // WinUI 3 Light CardBackgroundFillColorDefault / ControlFillColor*
                card_bg: rgba(255, 255, 255, 0.70),
                card_bg_hover: rgba(249, 249, 249, 0.88),
                card_bg_pressed: rgba(249, 249, 249, 0.52),
                card_drawer_bg: rgba(0, 0, 0, 0.024),
                card_border: rgba(0, 0, 0, 0.058),
                // Bottom bevel shadow in Light Mode (ControlElevationBorder)
                elevation_edge: rgba(0, 0, 0, 0.162),
                icon_well: rgba(0, 0, 0, 0.038),
                icon_stroke: rgba(0, 0, 0, 0.84),
                control_bg: rgba(255, 255, 255, 0.72),
                control_bg_hover: rgba(249, 249, 249, 0.58),
                control_bg_pressed: rgba(249, 249, 249, 0.36),
                control_border: rgba(0, 0, 0, 0.065),
                subtle_hover: rgba(0, 0, 0, 0.037),
                subtle_pressed: rgba(0, 0, 0, 0.024),
                flyout_bg: rgba(252, 252, 252, 0.99),
                flyout_border: rgba(0, 0, 0, 0.095),
                flyout_shadow: rgba(0, 0, 0, 0.12),
                compact_text_shadow: rgba(255, 255, 255, 0.65),
                text_primary: rgba(0, 0, 0, 0.896),
                text_secondary: rgba(0, 0, 0, 0.606),
                text_subtle: rgba(0, 0, 0, 0.446),
                accent_fill,
                accent_fill_hover,
                accent_text,
                accent_subtle_bg,
                accent_subtle_border,
                text_on_accent,
                bar_track: rgba(0, 0, 0, 0.12),
                // Official WinUI 3 Light semantic status colors
                status_good: rgba(15, 123, 15, 1.0),
                status_good_muted: rgba(15, 123, 15, 0.45),
                status_caution: rgba(157, 93, 0, 1.0),
                status_critical: rgba(196, 43, 28, 1.0),
                toggle_off_bg: rgba(0, 0, 0, 0.024),
                toggle_off_bg_hover: rgba(0, 0, 0, 0.058),
                toggle_off_border: rgba(0, 0, 0, 0.446),
                toggle_off_thumb: rgba(0, 0, 0, 0.606),
                toast_bg: rgba(255, 255, 255, 0.96),
                toast_border: rgba(0, 0, 0, 0.10),
                toast_text: rgba(0, 0, 0, 0.896),
                scrollbar_thumb: rgba(0, 0, 0, 0.30),
            }
        }
    }
}

/// Derive WinUI 3 contrast-safe accent tokens from the raw Windows system accent color.
fn derive_accent_tokens(
    dark: bool,
    rgb: (u8, u8, u8),
) -> (
    D2D1_COLOR_F,
    D2D1_COLOR_F,
    D2D1_COLOR_F,
    D2D1_COLOR_F,
    D2D1_COLOR_F,
    D2D1_COLOR_F,
) {
    let (h, s, l) = rgb_to_hsl(rgb);
    let fill_rgb = if dark {
        // WinUI 3 `SystemAccentColorLight2`: luminous on dark charcoal surfaces.
        hsl_to_rgb(h, s.clamp(0.45, 0.95), l.clamp(0.64, 0.74))
    } else {
        // WinUI 3 `SystemAccentColorDark1`: crisp & accessible on light surfaces.
        hsl_to_rgb(h, s.clamp(0.50, 0.96), l.clamp(0.34, 0.43))
    };
    let hover_rgb = if dark {
        hsl_to_rgb(
            h,
            s.clamp(0.45, 0.95),
            (l.clamp(0.64, 0.74) - 0.05).max(0.54),
        )
    } else {
        hsl_to_rgb(
            h,
            s.clamp(0.50, 0.96),
            (l.clamp(0.34, 0.43) + 0.05).min(0.50),
        )
    };
    let text_rgb = if dark {
        hsl_to_rgb(h, s.clamp(0.45, 0.95), l.clamp(0.70, 0.78))
    } else {
        hsl_to_rgb(h, s.clamp(0.50, 0.96), l.clamp(0.28, 0.38))
    };

    let accent_fill = rgba(fill_rgb.0, fill_rgb.1, fill_rgb.2, 1.0);
    let accent_fill_hover = rgba(hover_rgb.0, hover_rgb.1, hover_rgb.2, 1.0);
    let accent_text = rgba(text_rgb.0, text_rgb.1, text_rgb.2, 1.0);
    let accent_subtle_bg = rgba(
        fill_rgb.0,
        fill_rgb.1,
        fill_rgb.2,
        if dark { 0.16 } else { 0.11 },
    );
    let accent_subtle_border = rgba(
        fill_rgb.0,
        fill_rgb.1,
        fill_rgb.2,
        if dark { 0.34 } else { 0.26 },
    );
    let text_on_accent = if relative_luminance(fill_rgb) > 0.36 {
        rgba(0, 0, 0, 0.92)
    } else {
        rgba(255, 255, 255, 1.0)
    };

    (
        accent_fill,
        accent_fill_hover,
        accent_text,
        accent_subtle_bg,
        accent_subtle_border,
        text_on_accent,
    )
}

fn relative_luminance((r, g, b): (u8, u8, u8)) -> f32 {
    let lin = |c: u8| {
        let v = (c as f32) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

fn rgb_to_hsl((r, g, b): (u8, u8, u8)) -> (f32, f32, f32) {
    let rf = (r as f32) / 255.0;
    let gf = (g as f32) / 255.0;
    let bf = (b as f32) / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) * 0.5;
    let d = max - min;
    if d.abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let mut h = if (max - rf).abs() < 1e-6 {
        (gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }
    } else if (max - gf).abs() < 1e-6 {
        (bf - rf) / d + 2.0
    } else {
        (rf - gf) / d + 4.0
    };
    h /= 6.0;
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s <= 1e-6 {
        let v = (l * 255.0).round().clamp(0.0, 255.0) as u8;
        return (v, v, v);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
    (
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: (r as f32) / 255.0,
        g: (g as f32) / 255.0,
        b: (b as f32) / 255.0,
        a,
    }
}

/// Linearly interpolate two `D2D1_COLOR_F` colors with clamped `t` in `0.0..=1.0`.
pub fn lerp_color(a: D2D1_COLOR_F, b: D2D1_COLOR_F, t: f32) -> D2D1_COLOR_F {
    let t = t.clamp(0.0, 1.0);
    D2D1_COLOR_F {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Scale a color's alpha channel by `factor` (0.0..=1.0).
pub fn with_alpha(mut c: D2D1_COLOR_F, factor: f32) -> D2D1_COLOR_F {
    c.a *= factor.clamp(0.0, 1.0);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_effective_dark_mode_for_all_theme_modes() {
        let dark_sys = SystemTheme {
            app_dark: true,
            taskbar_dark: true,
            transparency_enabled: true,
            accent_rgb: DEFAULT_ACCENT_RGB,
        };
        assert!(dark_sys.resolve_dark(ThemeMode::System));
        assert!(!dark_sys.resolve_dark(ThemeMode::Light));
        assert!(dark_sys.resolve_dark(ThemeMode::Dark));

        let light_sys = SystemTheme {
            app_dark: false,
            taskbar_dark: false,
            transparency_enabled: true,
            accent_rgb: DEFAULT_ACCENT_RGB,
        };
        assert!(!light_sys.resolve_dark(ThemeMode::System));
        assert!(!light_sys.resolve_dark(ThemeMode::Light));
        assert!(light_sys.resolve_dark(ThemeMode::Dark));
    }

    #[test]
    fn palette_accent_contrast_matches_winui3_expectations() {
        let dark_pal = Palette::for_theme(true, DEFAULT_ACCENT_RGB);
        // Lightened accent on dark background uses dark text on accent.
        assert!(dark_pal.text_on_accent.r < 0.2);

        let light_pal = Palette::for_theme(false, DEFAULT_ACCENT_RGB);
        // Darkened accent on light background uses white text on accent.
        assert!(light_pal.text_on_accent.r > 0.9);
    }
}
