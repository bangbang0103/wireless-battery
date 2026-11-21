//! Native Windows application & system tray HICON loader and vector renderer.
//!
//! - Application/window icon (`load_app_icon`): loads the multi-resolution brushed-titanium
//!   `app-icon.ico` embedded in PE resources by `build.rs`.
//! - System tray icon (`build_tray_logo_icon`): renders the minimalist battery + wireless
//!   signal SVG geometry (`tray/assets/tray-normal.svg` / `tray-low.svg`) natively via
//!   Direct2D at the exact DPI-scaled system tray size (`SM_CXSMICON`), adapting to
//!   Windows light/dark taskbar and `TrayIconState` (`Normal` / `Idle` / `LowBattery`).

use crate::theme::rgba;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{BOOL, HMODULE, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_OPEN,
    D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F, D2D_SIZE_F,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, ID2D1PathGeometry, ID2D1StrokeStyle,
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_SMALL, D2D1_CAP_STYLE_ROUND,
    D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_LINE_JOIN_ROUND,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_ROUNDED_RECT,
    D2D1_STROKE_STYLE_PROPERTIES, D2D1_SWEEP_DIRECTION_CLOCKWISE,
};
use windows::Win32::Graphics::DirectWrite::IDWriteFactory;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, GetSystemMetrics, LoadIconW, HICON, ICONINFO, SM_CXSMICON,
};
use wireless_battery_tray_core::TrayIconState;

/// Win32 `RT_GROUP_ICON` resource ID embedded by `build.rs`.
const APP_ICON_RESOURCE_ID: u16 = 1;

/// Load the embedded multi-resolution application icon from the executable's PE resources.
pub fn load_app_icon(hinstance: HMODULE) -> HICON {
    unsafe {
        LoadIconW(
            hinstance,
            PCWSTR(usize::from(APP_ICON_RESOURCE_ID) as *const u16),
        )
        .unwrap_or_default()
    }
}

/// Owning wrapper around a Win32 `HICON` handle.
pub struct OwnedIcon {
    handle: HICON,
}

impl OwnedIcon {
    pub fn handle(&self) -> HICON {
        self.handle
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = DestroyIcon(self.handle);
            }
        }
    }
}

/// Choose `Segoe Fluent Icons` on Windows 11, falling back gracefully to `Segoe MDL2 Assets`.
pub fn resolve_icon_font_family(dwrite: &IDWriteFactory) -> PCWSTR {
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
                    .FindFamilyName(w!("Segoe Fluent Icons"), &mut index, &mut exists)
                    .is_ok()
                    && exists.as_bool()
                {
                    return w!("Segoe Fluent Icons");
                }
            }
        }
    }
    w!("Segoe MDL2 Assets")
}

/// Generate a crisp DPI-matched minimalist SVG vector tray icon (battery + wireless arcs).
pub fn build_tray_logo_icon(state: TrayIconState, taskbar_dark: bool) -> Option<OwnedIcon> {
    let size = tray_icon_pixel_size();
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    unsafe {
        let mem_dc: HDC = CreateCompatibleDC(HDC::default());
        if mem_dc.is_invalid() {
            return None;
        }

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let Ok(color_bmp): windows::core::Result<HBITMAP> =
            CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
        else {
            let _ = DeleteDC(mem_dc);
            return None;
        };

        let old_obj = SelectObject(mem_dc, color_bmp);
        let rendered = render_minimal_tray_svg(mem_dc, size, state, taskbar_dark).is_ok();
        SelectObject(mem_dc, old_obj);
        let _ = DeleteDC(mem_dc);

        if !rendered {
            let _ = DeleteObject(color_bmp);
            return None;
        }

        let mask_bmp: HBITMAP = CreateBitmap(size, size, 1, 1, None);
        if mask_bmp.is_invalid() {
            let _ = DeleteObject(color_bmp);
            return None;
        }

        let info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bmp,
            hbmColor: color_bmp,
        };
        let icon = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(color_bmp);
        let _ = DeleteObject(mask_bmp);

        icon.map(|handle| OwnedIcon { handle })
    }
}

fn tray_icon_pixel_size() -> i32 {
    let metric = unsafe { GetSystemMetrics(SM_CXSMICON) };
    if metric <= 0 {
        32
    } else {
        metric.clamp(16, 64)
    }
}

/// Render the minimalist 24x24 battery + Wi-Fi SVG geometry cleanly onto `dc`.
fn render_minimal_tray_svg(
    dc: HDC,
    size: i32,
    state: TrayIconState,
    taskbar_dark: bool,
) -> windows::core::Result<()> {
    unsafe {
        let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            ..Default::default()
        };
        let rt: ID2D1DCRenderTarget = d2d.CreateDCRenderTarget(&props)?;
        let bounds = RECT {
            left: 0,
            top: 0,
            right: size,
            bottom: size,
        };
        rt.BindDC(dc, &bounds)?;
        rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);

        let stroke_props = D2D1_STROKE_STYLE_PROPERTIES {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_ROUND,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            ..Default::default()
        };
        let round_stroke: ID2D1StrokeStyle = d2d.CreateStrokeStyle(&stroke_props, None)?;

        let frame_alpha = match state {
            TrayIconState::Idle => 0.48,
            TrayIconState::Normal | TrayIconState::LowBattery => 0.98,
        };
        let frame_color = if taskbar_dark {
            rgba(248, 250, 252, frame_alpha)
        } else {
            rgba(20, 28, 42, frame_alpha)
        };
        let signal_color = match state {
            TrayIconState::Idle => frame_color,
            TrayIconState::Normal => {
                if taskbar_dark {
                    rgba(52, 211, 153, 1.0)
                } else {
                    rgba(5, 150, 105, 1.0)
                }
            }
            TrayIconState::LowBattery => rgba(244, 63, 94, 1.0),
        };

        let frame_brush = rt.CreateSolidColorBrush(&frame_color, None)?;
        let signal_brush = rt.CreateSolidColorBrush(&signal_color, None)?;

        let s = (size as f32) / 24.0;
        let body_stroke = (1.8 * s).max(1.25);
        let wave_stroke = (1.7 * s).max(1.15);

        let outer_arc = create_wifi_arc(&d2d, 6.75 * s, 10.44 * s, 14.25 * s, 10.44 * s, 5.6 * s)?;
        let inner_arc = create_wifi_arc(&d2d, 8.36 * s, 12.22 * s, 12.64 * s, 12.22 * s, 3.2 * s)?;

        rt.BeginDraw();
        rt.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        // Minimalist horizontal battery outline (viewBox 1.5, 5.5, 18 x 13, rx 2.8).
        let battery_body = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: 1.5 * s,
                top: 5.5 * s,
                right: 19.5 * s,
                bottom: 18.5 * s,
            },
            radiusX: 2.8 * s,
            radiusY: 2.8 * s,
        };
        rt.DrawRoundedRectangle(&battery_body, &frame_brush, body_stroke, &round_stroke);

        // Positive terminal pill on the right (viewBox 21.0, 9.2, 2.0 x 5.6, rx 1.0).
        let terminal = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: 21.0 * s,
                top: 9.2 * s,
                right: 23.0 * s,
                bottom: 14.8 * s,
            },
            radiusX: 1.0 * s,
            radiusY: 1.0 * s,
        };
        rt.FillRoundedRectangle(&terminal, &frame_brush);

        // Minimalist wireless Wi-Fi waves + base dot inside the battery silhouette.
        rt.DrawGeometry(&outer_arc, &signal_brush, wave_stroke, &round_stroke);
        rt.DrawGeometry(&inner_arc, &signal_brush, wave_stroke, &round_stroke);
        rt.FillEllipse(
            &D2D1_ELLIPSE {
                point: D2D_POINT_2F {
                    x: 10.5 * s,
                    y: 14.6 * s,
                },
                radiusX: (1.15 * s).max(0.85),
                radiusY: (1.15 * s).max(0.85),
            },
            &signal_brush,
        );

        // Coral-red alert badge dot when any peripheral is low battery.
        if state == TrayIconState::LowBattery {
            let dot_color = rgba(244, 63, 94, 1.0);
            let dot_brush = rt.CreateSolidColorBrush(&dot_color, None)?;
            rt.FillEllipse(
                &D2D1_ELLIPSE {
                    point: D2D_POINT_2F {
                        x: 20.2 * s,
                        y: 4.2 * s,
                    },
                    radiusX: (2.4 * s).max(1.8),
                    radiusY: (2.4 * s).max(1.8),
                },
                &dot_brush,
            );
        }

        rt.EndDraw(None, None)
    }
}

unsafe fn create_wifi_arc(
    d2d: &ID2D1Factory,
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    radius: f32,
) -> windows::core::Result<ID2D1PathGeometry> {
    let geometry = d2d.CreatePathGeometry()?;
    let sink = geometry.Open()?;
    sink.BeginFigure(
        D2D_POINT_2F {
            x: start_x,
            y: start_y,
        },
        D2D1_FIGURE_BEGIN_HOLLOW,
    );
    sink.AddArc(&D2D1_ARC_SEGMENT {
        point: D2D_POINT_2F { x: end_x, y: end_y },
        size: D2D_SIZE_F {
            width: radius,
            height: radius,
        },
        rotationAngle: 0.0,
        sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
        arcSize: D2D1_ARC_SIZE_SMALL,
    });
    sink.EndFigure(D2D1_FIGURE_END_OPEN);
    sink.Close()?;
    Ok(geometry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_minimal_svg_tray_icons_for_all_states() {
        for &dark in &[true, false] {
            for state in [
                TrayIconState::Idle,
                TrayIconState::Normal,
                TrayIconState::LowBattery,
            ] {
                let icon = build_tray_logo_icon(state, dark);
                assert!(
                    icon.is_some(),
                    "failed to render tray icon for state={state:?}, dark={dark}"
                );
            }
        }
    }
}
