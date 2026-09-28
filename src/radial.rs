use orbit::geometry::{Action, Rect};
use orbit::settings::Settings;
use windows::Win32::Foundation::{COLORREF, HWND, POINT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HGDIOBJ,
    SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};

const CANVAS_GUTTER_PT: u32 = 80;

#[derive(Clone, Debug)]
pub struct RadialBitmap {
    pub size: i32,
    /// Premultiplied BGRA pixels in top-to-bottom row order.
    pub pixels: Vec<u32>,
}

/// Return the ring diameter in physical pixels for a monitor/window DPI.
pub fn radial_size_px(settings: &Settings, dpi: u32) -> i32 {
    ((settings.radial_size.clamp(1, 512) as u64 * u64::from(dpi.max(48)) + 48) / 96).clamp(1, 4096)
        as i32
}

/// The layered window keeps transparent room around the ring for the center glyph and its edges.
pub fn window_size_px(settings: &Settings, dpi: u32) -> i32 {
    let logical = settings.radial_size.clamp(1, 512) + CANVAS_GUTTER_PT;
    ((logical as u64 * u64::from(dpi.max(48)) + 48) / 96).clamp(1, 4096) as i32
}

/// Render an action selection without creating a window. If the same action is assigned to
/// multiple directions, each matching direction is highlighted.
pub fn render_bitmap(
    settings: &Settings,
    selected: Option<Action>,
    dpi: u32,
) -> Result<RadialBitmap, String> {
    let mut selected_slots = [false; 8];
    if let Some(action) = selected {
        for (slot, candidate) in settings.radial_actions.iter().enumerate() {
            selected_slots[slot] = *candidate == action;
        }
    }
    render(settings, selected, selected_slots, dpi)
}

/// Render one selected direction without creating a window.
#[cfg(test)]
pub fn render_bitmap_for_sector(
    settings: &Settings,
    selected_sector: Option<usize>,
    dpi: u32,
) -> Result<RadialBitmap, String> {
    if selected_sector.is_some_and(|sector| sector >= settings.radial_actions.len()) {
        return Err("selected radial sector is outside the eight menu directions".into());
    }
    let mut selected_slots = [false; 8];
    if let Some(sector) = selected_sector {
        selected_slots[sector] = true;
    }
    let selected = selected_sector.map(|sector| settings.radial_actions[sector]);
    render(settings, selected, selected_slots, dpi)
}

/// Draw the radial overlay as a premultiplied BGRA layered bitmap.
pub fn draw_with_settings(
    window: HWND,
    selected: Option<Action>,
    settings: &Settings,
    dpi: u32,
) -> Result<(), String> {
    let bitmap = render_bitmap(settings, selected, dpi)?;
    update_layered_window(window, bitmap)
}

/// Draw exactly one menu direction. Use this when the runtime tracks sector identity, so repeated
/// actions in different directions remain visually distinct.
pub fn draw_with_settings_and_sector(
    window: HWND,
    selected_action: Option<Action>,
    selected_sector: Option<usize>,
    settings: &Settings,
    dpi: u32,
) -> Result<(), String> {
    if selected_sector.is_some_and(|sector| sector >= settings.radial_actions.len()) {
        return Err("selected radial sector is outside the eight menu directions".into());
    }
    let mut selected_slots = [false; 8];
    if let Some(sector) = selected_sector {
        selected_slots[sector] = true;
    }
    let bitmap = render(settings, selected_action, selected_slots, dpi)?;
    update_layered_window(window, bitmap)
}

fn render(
    settings: &Settings,
    selected_action: Option<Action>,
    selected_slots: [bool; 8],
    dpi: u32,
) -> Result<RadialBitmap, String> {
    let dpi = dpi.max(48);
    let size = window_size_px(settings, dpi);
    let scale = f64::from(dpi) / 96.0;
    let center = f64::from(size) / 2.0;
    let outer = f64::from(radial_size_px(settings, dpi)) / 2.0;
    let thickness =
        (f64::from(settings.radial_thickness.max(1)) * scale).clamp(1.0, outer.max(1.0));
    let inner = (outer - thickness).max(0.0);
    let outer_corner = (f64::from(settings.radial_corner_radius) * scale).min(outer);
    let inner_corner = (outer_corner - thickness).max(0.0).min(inner);
    let selected_color = [
        ((settings.accent_color >> 16) & 0xff) as f64,
        ((settings.accent_color >> 8) & 0xff) as f64,
        (settings.accent_color & 0xff) as f64,
    ];
    let pixel_count = usize::try_from(size)
        .ok()
        .and_then(|value| value.checked_mul(value))
        .ok_or_else(|| "radial menu is too large to render".to_string())?;
    let mut pixels = vec![0u32; pixel_count];

    for y in 0..size {
        for x in 0..size {
            let dx = f64::from(x) + 0.5 - center;
            let dy = f64::from(y) + 0.5 - center;
            let outer_distance = rounded_box_distance(dx, dy, outer, outer, outer_corner);
            let inner_distance = rounded_box_distance(dx, dy, inner, inner, inner_corner);
            // Intersect rounded outer coverage with the inverse rounded inner shape. Radius 0
            // produces a square-corner ring; radius equal to half the diameter produces a circle.
            let ring_coverage = ((-outer_distance).min(inner_distance) + 0.5).clamp(0.0, 1.0);
            let mut color = [44.0, 49.0, 59.0];
            let mut alpha = ring_coverage * 240.0;

            if let Some(action) = selected_action {
                let angle = dy.atan2(dx);
                let slot_count = settings.radial_actions.len() as f64;
                for (slot, selected) in selected_slots.iter().enumerate() {
                    if !selected {
                        continue;
                    }
                    let center_angle = slot as f64 * std::f64::consts::TAU / slot_count;
                    let delta = (angle - center_angle + std::f64::consts::PI)
                        .rem_euclid(std::f64::consts::TAU)
                        - std::f64::consts::PI;
                    let angular_distance =
                        (std::f64::consts::FRAC_PI_8 - delta.abs()) * dx.hypot(dy);
                    let sector_coverage = (angular_distance + 0.5).clamp(0.0, 1.0);
                    if sector_coverage > 0.0 && ring_coverage > 0.0 {
                        color = selected_color;
                        alpha = (ring_coverage * sector_coverage * 255.0).max(alpha);
                        break;
                    }
                }

                // Keep the center glyph independent of ring coverage; it is visible in the
                // transparent center and still reflects the selected frame.
                if let Some(glyph) = glyph_pixel(x, y, center, scale, action, selected_color) {
                    color = glyph.0;
                    alpha = glyph.1;
                }
            }

            pixels[(y * size + x) as usize] = premultiplied_bgra(color, alpha);
        }
    }

    Ok(RadialBitmap { size, pixels })
}

fn update_layered_window(window: HWND, bitmap: RadialBitmap) -> Result<(), String> {
    let size = bitmap.size;
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.0.is_null() {
        return Err("cannot create radial drawing context".into());
    }
    let info = BITMAPINFO {
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
    let mut bits = std::ptr::null_mut();
    let bitmap_handle =
        match unsafe { CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) } {
            Ok(bitmap) => bitmap,
            Err(error) => {
                unsafe {
                    let _ = DeleteDC(dc);
                }
                return Err(error.to_string());
            }
        };
    if bits.is_null() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(bitmap_handle.0));
            let _ = DeleteDC(dc);
        }
        return Err("cannot allocate radial menu bitmap".into());
    }

    let result = unsafe {
        std::ptr::copy_nonoverlapping(
            bitmap.pixels.as_ptr(),
            bits.cast::<u32>(),
            bitmap.pixels.len(),
        );
        let previous = SelectObject(dc, HGDIOBJ(bitmap_handle.0));
        let result = UpdateLayeredWindow(
            window,
            None,
            None,
            Some(&SIZE { cx: size, cy: size }),
            Some(dc),
            Some(&POINT::default()),
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
        let _ = DeleteObject(HGDIOBJ(bitmap_handle.0));
        let _ = DeleteDC(dc);
        result
    };
    result.map_err(|error| format!("cannot render radial menu: {error}"))
}

fn rounded_box_distance(x: f64, y: f64, half_width: f64, half_height: f64, radius: f64) -> f64 {
    let radius = radius.min(half_width).min(half_height);
    let qx = x.abs() - half_width + radius;
    let qy = y.abs() - half_height + radius;
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

fn glyph_pixel(
    x: i32,
    y: i32,
    center: f64,
    scale: f64,
    action: Action,
    color: [f64; 3],
) -> Option<([f64; 3], f64)> {
    let glyph_width = (28.0 * scale).round() as i32;
    let glyph_height = (20.0 * scale).round() as i32;
    let left = center.round() as i32 - glyph_width / 2;
    let top = center.round() as i32 - glyph_height / 2;
    if x < left || x >= left + glyph_width || y < top || y >= top + glyph_height {
        return None;
    }
    let border =
        x == left || x == left + glyph_width - 1 || y == top || y == top + glyph_height - 1;
    let inset = (2.0 * scale).round() as i32;
    let inner_inset = (3.0 * scale).round() as i32;
    let inner_rect = Rect {
        left: left + inner_inset,
        top: top + inner_inset,
        right: left + glyph_width - inner_inset,
        bottom: top + glyph_height - inner_inset,
    };
    let frame = action.frame(
        Rect {
            left: left + inset,
            top: top + inset,
            right: left + glyph_width - inset,
            bottom: top + glyph_height - inset,
        },
        inner_rect,
        0,
    );
    let filled = x >= frame.left && x < frame.right && y >= frame.top && y < frame.bottom;
    (border || filled).then_some((color, if border { 210.0 } else { 255.0 }))
}

fn premultiplied_bgra(color: [f64; 3], alpha: f64) -> u32 {
    let channel = |value: f64| (value * alpha / 255.0).round() as u32;
    let alpha = alpha.round() as u32;
    (alpha << 24) | (channel(color[0]) << 16) | (channel(color[1]) << 8) | channel(color[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(bitmap: &RadialBitmap, x: i32, y: i32) -> u32 {
        bitmap.pixels[(y * bitmap.size + x) as usize]
    }

    #[test]
    fn renders_transparent_center_and_antialiased_ring_at_scaled_size() {
        let settings = Settings::default();
        let bitmap = render_bitmap(&settings, None, 96).unwrap();
        assert_eq!(bitmap.size, 180);
        assert_eq!(at(&bitmap, 90, 90) >> 24, 0);
        assert!(at(&bitmap, 130, 90) >> 24 > 0);
        assert_eq!(at(&bitmap, 150, 90) >> 24, 0);

        let scaled = render_bitmap(&settings, None, 144).unwrap();
        assert_eq!(scaled.size, 270);
        assert!(at(&scaled, 195, 135) >> 24 > 0);
    }

    #[test]
    fn corner_radius_selects_square_or_circular_ring_shape() {
        let mut settings = Settings {
            radial_corner_radius: 0,
            ..Settings::default()
        };
        let square = render_bitmap(&settings, None, 96).unwrap();
        assert!(at(&square, 139, 139) >> 24 > 0);

        settings.radial_corner_radius = 50;
        let circle = render_bitmap(&settings, None, 96).unwrap();
        assert_eq!(at(&circle, 139, 139) >> 24, 0);
    }

    #[test]
    fn selected_sector_uses_accent_color_and_center_glyph() {
        let settings = Settings::default();
        let bitmap = render_bitmap_for_sector(&settings, Some(0), 96).unwrap();
        assert_eq!(at(&bitmap, 130, 90) & 0x00ff_ffff, 0x0067_c1d6);
        assert_ne!(at(&bitmap, 90, 80) >> 24, 0);
        assert!(render_bitmap_for_sector(&settings, Some(8), 96).is_err());
    }
}
