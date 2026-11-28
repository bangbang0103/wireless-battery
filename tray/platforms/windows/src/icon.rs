//! Native Windows application & system tray HICON loader and vector renderer.
//!
//! - Application/window icon (`load_app_icon`): loads the multi-resolution
//!   `app-icon.ico` embedded in PE resources by `build.rs`.
//! - System tray icon (`build_tray_logo_icon`): renders the rounded diagonal
//!   lightning + wireless broadcast SVG geometry (`tray/assets/tray-normal.svg` /
//!   `tray-low.svg`) natively via Direct2D at the exact DPI-scaled system tray
//!   size (`SM_CXSMICON`), adapting to Windows light/dark taskbar and
//!   `TrayIconState` (`Normal` / `Idle` / `LowBattery`).

use crate::theme::rgba;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{BOOL, HMODULE, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN, D2D1_PIXEL_FORMAT,
    D2D_POINT_2F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap, ID2D1DCRenderTarget, ID2D1Factory, ID2D1HwndRenderTarget,
    ID2D1PathGeometry, ID2D1StrokeStyle, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_BITMAP_PROPERTIES,
    D2D1_CAP_STYLE_ROUND, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_LINE_JOIN_ROUND,
    D2D1_QUADRATIC_BEZIER_SEGMENT, D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
    D2D1_STROKE_STYLE_PROPERTIES,
};
use windows::Win32::Graphics::DirectWrite::IDWriteFactory;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, DrawIconEx, GetSystemMetrics, LoadIconW, LoadImageW,
    DI_NORMAL, HICON, ICONINFO, IMAGE_ICON, LR_DEFAULTCOLOR, SM_CXSMICON,
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

/// Generate a crisp DPI-matched minimalist SVG vector tray icon (lightning + wireless arcs).
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

/// Render the 24x24 rounded diagonal lightning + wireless broadcast SVG geometry onto `dc`.
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

        let bolt_alpha = match state {
            TrayIconState::Idle => 0.48,
            TrayIconState::Normal | TrayIconState::LowBattery => 0.98,
        };
        let bolt_color = if taskbar_dark {
            rgba(248, 250, 252, bolt_alpha)
        } else {
            rgba(20, 28, 42, bolt_alpha)
        };
        let wave_color = match state {
            TrayIconState::LowBattery => rgba(244, 63, 94, 1.0),
            TrayIconState::Idle | TrayIconState::Normal => bolt_color,
        };

        let bolt_brush = rt.CreateSolidColorBrush(&bolt_color, None)?;
        let wave_brush = rt.CreateSolidColorBrush(&wave_color, None)?;

        let s = (size as f32) / 24.0;
        let inner_wave_stroke = (2.1 * s).max(1.25);
        let outer_wave_stroke = (1.8 * s).max(1.1);

        let origin = D2D_POINT_2F { x: 0.0, y: 0.0 };
        let bolt_geometry = create_rounded_bolt_geometry(&d2d, s, origin)?;
        let inner_arc = create_quad_curve(
            &d2d,
            15.9 * s,
            5.2 * s,
            19.6 * s,
            8.0 * s,
            19.1 * s,
            12.0 * s,
        )?;
        let outer_arc = create_quad_curve(
            &d2d,
            16.8 * s,
            2.0 * s,
            22.6 * s,
            6.0 * s,
            21.6 * s,
            13.0 * s,
        )?;

        rt.BeginDraw();
        rt.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        rt.FillGeometry(&bolt_geometry, &bolt_brush, None);
        rt.DrawGeometry(&inner_arc, &wave_brush, inner_wave_stroke, &round_stroke);
        rt.DrawGeometry(&outer_arc, &wave_brush, outer_wave_stroke, &round_stroke);

        rt.EndDraw(None, None)
    }
}

/// Rasterize the embedded application icon (`app-icon.ico`, built from `app-icon.png`)
/// into a Direct2D bitmap at the exact DPI-scaled pixel size for the popover header.
pub fn create_app_icon_bitmap(rt: &ID2D1HwndRenderTarget, pixel_size: u32) -> Option<ID2D1Bitmap> {
    let size = pixel_size.clamp(16, 128) as i32;
    unsafe {
        let hinstance = GetModuleHandleW(None).ok()?;
        let handle = LoadImageW(
            hinstance,
            PCWSTR(usize::from(APP_ICON_RESOURCE_ID) as *const u16),
            IMAGE_ICON,
            size,
            size,
            LR_DEFAULTCOLOR,
        )
        .ok()?;
        let hicon = HICON(handle.0);
        if hicon.is_invalid() {
            return None;
        }

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

        let mem_dc: HDC = CreateCompatibleDC(HDC::default());
        if mem_dc.is_invalid() {
            let _ = DestroyIcon(hicon);
            return None;
        }

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let Ok(dib): windows::core::Result<HBITMAP> =
            CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
        else {
            let _ = DeleteDC(mem_dc);
            let _ = DestroyIcon(hicon);
            return None;
        };
        if bits.is_null() {
            let _ = DeleteObject(dib);
            let _ = DeleteDC(mem_dc);
            let _ = DestroyIcon(hicon);
            return None;
        }

        let old_obj = SelectObject(mem_dc, dib);
        let draw_ok = DrawIconEx(mem_dc, 0, 0, hicon, size, size, 0, None, DI_NORMAL).is_ok();
        let _ = GdiFlush();

        let bitmap = if draw_ok {
            let props = D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            };
            rt.CreateBitmap(
                D2D_SIZE_U {
                    width: size as u32,
                    height: size as u32,
                },
                Some(bits.cast_const()),
                (size as u32) * 4,
                &props,
            )
            .ok()
        } else {
            None
        };

        SelectObject(mem_dc, old_obj);
        let _ = DeleteObject(dib);
        let _ = DeleteDC(mem_dc);
        let _ = DestroyIcon(hicon);
        bitmap
    }
}

/// Centered 70%-rounded diagonal lightning bolt geometry in 24x24 viewBox coordinates.
unsafe fn create_rounded_bolt_geometry(
    d2d: &ID2D1Factory,
    s: f32,
    offset: D2D_POINT_2F,
) -> windows::core::Result<ID2D1PathGeometry> {
    type Pt = (f32, f32);
    type Corner = (Pt, Pt, Pt);
    const CORNERS: [Corner; 6] = [
        ((11.27, 3.89), (11.60, 2.40), (10.71, 3.64)),
        ((3.37, 13.92), (2.60, 15.00), (3.93, 15.00)),
        ((6.77, 15.00), (8.10, 15.00), (7.73, 16.28)),
        ((6.52, 20.53), (6.10, 22.00), (7.01, 20.77)),
        ((14.81, 10.17), (15.60, 9.10), (14.27, 9.10)),
        ((11.43, 9.10), (10.10, 9.10), (10.39, 7.80)),
    ];

    let geometry = d2d.CreatePathGeometry()?;
    let sink = geometry.Open()?;
    let (start_x, start_y) = CORNERS[0].0;
    sink.BeginFigure(
        D2D_POINT_2F {
            x: offset.x + start_x * s,
            y: offset.y + start_y * s,
        },
        D2D1_FIGURE_BEGIN_FILLED,
    );
    for (idx, &(p1, ctrl, p2)) in CORNERS.iter().enumerate() {
        if idx > 0 {
            sink.AddLine(D2D_POINT_2F {
                x: offset.x + p1.0 * s,
                y: offset.y + p1.1 * s,
            });
        }
        sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
            point1: D2D_POINT_2F {
                x: offset.x + ctrl.0 * s,
                y: offset.y + ctrl.1 * s,
            },
            point2: D2D_POINT_2F {
                x: offset.x + p2.0 * s,
                y: offset.y + p2.1 * s,
            },
        });
    }
    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
    sink.Close()?;
    Ok(geometry)
}

unsafe fn create_quad_curve(
    d2d: &ID2D1Factory,
    start_x: f32,
    start_y: f32,
    ctrl_x: f32,
    ctrl_y: f32,
    end_x: f32,
    end_y: f32,
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
    sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
        point1: D2D_POINT_2F {
            x: ctrl_x,
            y: ctrl_y,
        },
        point2: D2D_POINT_2F { x: end_x, y: end_y },
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
