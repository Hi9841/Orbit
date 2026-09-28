use orbit::geometry::Action;
use orbit::settings::Settings;
use windows::Win32::Foundation::{COLORREF, HWND, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HGDIOBJ,
    SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewBitmap {
    pub width: i32,
    pub height: i32,
    /// Premultiplied BGRA pixels in top-to-bottom row order.
    pub pixels: Vec<u32>,
}

/// Draw a rounded preview surface with a fully opaque accent border and independently translucent
/// interior. `rect_size` is in physical pixels to match the preview HWND; DPI scales the logical
/// border and corner settings.
#[cfg(test)]
pub fn render_bitmap(
    settings: &Settings,
    action: Action,
    rect_size: (u32, u32),
    dpi: u32,
) -> Result<PreviewBitmap, String> {
    render_bitmap_with_colors(settings, action, rect_size, dpi, None)
}

/// Render with an optional system accent resolved by the platform layer.
pub fn render_bitmap_with_colors(
    settings: &Settings,
    _action: Action,
    rect_size: (u32, u32),
    dpi: u32,
    system_accent: Option<u32>,
) -> Result<PreviewBitmap, String> {
    let dpi = dpi.max(48);
    let scale = f64::from(dpi) / 96.0;
    let width = u64::from(rect_size.0);
    let height = u64::from(rect_size.1);
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
        return Err("preview dimensions are outside the supported range".into());
    }
    let pixel_count = width
        .checked_mul(height)
        .filter(|count| *count <= 33_554_432)
        .ok_or_else(|| "preview bitmap would exceed the safe render limit".to_string())?;
    let width = width as i32;
    let height = height as i32;
    let half_width = f64::from(width) / 2.0;
    let half_height = f64::from(height) / 2.0;
    let radius =
        (f64::from(settings.preview_corner_radius) * scale).min(half_width.min(half_height));
    let thickness =
        (f64::from(settings.preview_border_thickness) * scale).min(half_width.min(half_height));
    let inner_half_width = (half_width - thickness).max(0.0);
    let inner_half_height = (half_height - thickness).max(0.0);
    let inner_radius = (radius - thickness).max(0.0);
    let opacity = f64::from(settings.preview_opacity);
    let first: u32 = if settings.use_system_accent {
        system_accent.unwrap_or(settings.accent_color)
    } else {
        settings.accent_color
    } & 0x00ff_ffffu32;
    let second: u32 = if settings.use_gradient {
        settings.gradient_color & 0x00ff_ffffu32
    } else {
        first
    };
    let colors: Vec<[f64; 3]> = (0..width)
        .map(|x| {
            let position = if width <= 1 {
                0.5
            } else {
                f64::from(x) / f64::from(width - 1)
            };
            interpolate_rgb(first, second, position)
        })
        .collect();
    let mut pixels = vec![0; pixel_count as usize];

    for y in 0..height {
        for x in 0..width {
            let dx = f64::from(x) + 0.5 - half_width;
            let dy = f64::from(y) + 0.5 - half_height;
            let outer_distance = rounded_box_distance(dx, dy, half_width, half_height, radius);
            let outer_coverage = (0.5 - outer_distance).clamp(0.0, 1.0);
            if outer_coverage == 0.0 {
                continue;
            }
            let inner_coverage = if thickness == 0.0 {
                outer_coverage
            } else {
                let inner_distance =
                    rounded_box_distance(dx, dy, inner_half_width, inner_half_height, inner_radius);
                (0.5 - inner_distance).clamp(0.0, 1.0).min(outer_coverage)
            };
            let border_coverage = (outer_coverage - inner_coverage).max(0.0);
            let color = colors[x as usize];
            let alpha = (border_coverage * 255.0 + inner_coverage * opacity).clamp(0.0, 255.0);
            pixels[(y * width + x) as usize] = premultiplied_bgra(color, alpha);
        }
    }

    Ok(PreviewBitmap {
        width,
        height,
        pixels,
    })
}

/// Present a rendered preview through a per-pixel layered window.
pub fn update_layered_window(window: HWND, bitmap: &PreviewBitmap) -> Result<(), String> {
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.0.is_null() {
        return Err("cannot create preview drawing context".into());
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bitmap.width,
            biHeight: -bitmap.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let image =
        match unsafe { CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) } {
            Ok(image) => image,
            Err(error) => {
                unsafe {
                    let _ = DeleteDC(dc);
                }
                return Err(error.to_string());
            }
        };
    if bits.is_null() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(image.0));
            let _ = DeleteDC(dc);
        }
        return Err("cannot allocate preview bitmap".into());
    }

    let result = unsafe {
        std::ptr::copy_nonoverlapping(
            bitmap.pixels.as_ptr(),
            bits.cast::<u32>(),
            bitmap.pixels.len(),
        );
        let previous = SelectObject(dc, HGDIOBJ(image.0));
        let result = UpdateLayeredWindow(
            window,
            None,
            None,
            Some(&SIZE {
                cx: bitmap.width,
                cy: bitmap.height,
            }),
            Some(dc),
            None,
            COLORREF(0),
            Some(&BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            }),
            ULW_ALPHA,
        );
        let _ = SelectObject(dc, previous);
        let _ = DeleteObject(HGDIOBJ(image.0));
        let _ = DeleteDC(dc);
        result
    };
    result.map_err(|error| format!("cannot render preview: {error}"))
}

fn rounded_box_distance(x: f64, y: f64, half_width: f64, half_height: f64, radius: f64) -> f64 {
    let radius = radius.min(half_width).min(half_height);
    let qx = x.abs() - half_width + radius;
    let qy = y.abs() - half_height + radius;
    let outside_distance = if qx > 0.0 && qy > 0.0 {
        qx.hypot(qy)
    } else if qx > 0.0 {
        qx
    } else if qy > 0.0 {
        qy
    } else {
        0.0
    };
    outside_distance + qx.max(qy).min(0.0) - radius
}

fn interpolate_rgb(first: u32, second: u32, position: f64) -> [f64; 3] {
    let channel = |shift: u32| {
        let a = f64::from((first >> shift) & 0xffu32);
        let b = f64::from((second >> shift) & 0xffu32);
        a + (b - a) * position
    };
    [channel(16), channel(8), channel(0)]
}

fn premultiplied_bgra(color: [f64; 3], alpha: f64) -> u32 {
    let alpha = alpha.round() as u32;
    let channel = |value: f64| (value * f64::from(alpha) / 255.0).round() as u32;
    (alpha << 24) | (channel(color[0]) << 16) | (channel(color[1]) << 8) | channel(color[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(bitmap: &PreviewBitmap, x: i32, y: i32) -> u32 {
        bitmap.pixels[(y * bitmap.width + x) as usize]
    }

    #[test]
    fn renders_interior_opacity_and_independent_opaque_border() {
        let settings = Settings {
            preview_opacity: 128,
            preview_border_thickness: 4,
            ..Default::default()
        };
        let bitmap = render_bitmap(&settings, Action::LeftHalf, (40, 30), 96).unwrap();
        assert_eq!((bitmap.width, bitmap.height), (40, 30));
        assert_eq!(at(&bitmap, 20, 15) >> 24, 128);
        assert_eq!(at(&bitmap, 1, 15) >> 24, 255);
        assert_eq!(at(&bitmap, 0, 0) >> 24, 0);
    }

    #[test]
    fn gradient_interpolates_and_dpi_scales_the_bitmap_and_border() {
        let settings = Settings {
            use_gradient: true,
            accent_color: 0x000000,
            gradient_color: 0xffffff,
            preview_border_thickness: 2,
            preview_opacity: 100,
            preview_corner_radius: 0,
            ..Default::default()
        };
        let settings = Settings {
            preview_opacity: 128,
            ..settings
        };
        let bitmap = render_bitmap(&settings, Action::RightHalf, (30, 18), 144).unwrap();
        assert_eq!((bitmap.width, bitmap.height), (30, 18));
        assert!((at(&bitmap, 10, 9) & 0x00ff_ffff) < (at(&bitmap, 20, 9) & 0x00ff_ffff));
        assert_eq!(at(&bitmap, 2, 9) >> 24, 255);
        assert_eq!(at(&bitmap, 10, 9) >> 24, 128);
    }

    #[test]
    fn renderer_keeps_raw_alpha_byte_and_does_not_validate_unrelated_settings() {
        let settings = Settings {
            preview_opacity: 200,
            // A separate page may be mid-edit; the pure renderer only needs rendering fields.
            padding: -1,
            ..Default::default()
        };
        let bitmap = render_bitmap(&settings, Action::LeftHalf, (20, 20), 96).unwrap();
        assert_eq!(at(&bitmap, 10, 10) >> 24, 200);
    }
}
