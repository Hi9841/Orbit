use orbit::geometry::Action;
use orbit::settings::Settings;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewBitmap {
    pub width: i32,
    pub height: i32,
    /// Premultiplied BGRA pixels in top-to-bottom row order.
    pub pixels: Vec<u32>,
}

// A neutral tint approximates Loop's HUD material on Windows 10 and 11. The popup's layered
// opacity supplies desktop translucency; its accent wash stays restrained.
const HUD_COLOR: [f64; 3] = [32.0, 34.0, 38.0];
const HUD_ALPHA: f64 = 255.0;
const ACCENT_WASH_ALPHA: f64 = 25.5;
const EDGE_COLOR: [f64; 3] = [236.0, 236.0, 232.0];
const EDGE_ALPHA: f64 = 70.0;
const ACCENT_EDGE_ALPHA: f64 = 110.0;
const EDGE_PX: f64 = 1.0;

/// Draw a plate-sized preview. Callers that already have a window-sized surface use this.
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
#[cfg(test)]
pub fn render_bitmap_with_colors(
    settings: &Settings,
    action: Action,
    rect_size: (u32, u32),
    dpi: u32,
    system_accent: Option<u32>,
) -> Result<PreviewBitmap, String> {
    render_bitmap_with_backdrop(settings, action, rect_size, dpi, system_accent, None)
}

/// `backdrop` is a blurred, opaque BGRA capture of the destination rectangle.
/// It is sampled only inside the plate, so rounded corners remain click-through.
pub fn render_bitmap_with_backdrop(
    settings: &Settings,
    action: Action,
    rect_size: (u32, u32),
    dpi: u32,
    system_accent: Option<u32>,
    backdrop: Option<&[u32]>,
) -> Result<PreviewBitmap, String> {
    let width = i32::try_from(rect_size.0)
        .map_err(|_| "preview dimensions are outside the supported range".to_string())?;
    let height = i32::try_from(rect_size.1)
        .map_err(|_| "preview dimensions are outside the supported range".to_string())?;
    render_screen_preview_with_backdrop(
        settings,
        action,
        rect_size,
        (0, 0, width, height),
        dpi,
        system_accent,
        backdrop,
    )
}

/// Screen-sized placement preview. The plate matches Loop: rounded HUD fill, accent wash,
/// a 1 px light edge, and a DPI-scaled accent stroke. `frame` is `(x, y, width, height)`.
/// Pixels outside the plate are clear.
#[cfg(test)]
pub fn render_screen_preview(
    settings: &Settings,
    action: Action,
    screen_size: (u32, u32),
    frame: (i32, i32, i32, i32),
    dpi: u32,
    system_accent: Option<u32>,
) -> Result<PreviewBitmap, String> {
    render_screen_preview_with_backdrop(
        settings,
        action,
        screen_size,
        frame,
        dpi,
        system_accent,
        None,
    )
}

fn render_screen_preview_with_backdrop(
    settings: &Settings,
    action: Action,
    screen_size: (u32, u32),
    frame: (i32, i32, i32, i32),
    dpi: u32,
    system_accent: Option<u32>,
    backdrop: Option<&[u32]>,
) -> Result<PreviewBitmap, String> {
    let _ = action;
    let dpi = dpi.max(48);
    let scale = f64::from(dpi) / 96.0;
    let screen_w = u64::from(screen_size.0);
    let screen_h = u64::from(screen_size.1);
    if screen_w == 0 || screen_h == 0 || screen_w > 16_384 || screen_h > 16_384 {
        return Err("preview dimensions are outside the supported range".into());
    }
    let pixel_count = screen_w
        .checked_mul(screen_h)
        .filter(|count| *count <= 33_554_432)
        .ok_or_else(|| "preview bitmap would exceed the safe render limit".to_string())?;
    if backdrop.is_some_and(|pixels| pixels.len() != pixel_count as usize) {
        return Err("preview backdrop dimensions do not match the plate".into());
    }
    let (frame_x, frame_y, frame_w, frame_h) = frame;
    if frame_w <= 0 || frame_h <= 0 {
        return Err("preview frame is outside the supported range".into());
    }

    let width = screen_w as i32;
    let height = screen_h as i32;
    let half_width = f64::from(frame_w) / 2.0;
    let half_height = f64::from(frame_h) / 2.0;
    let radius =
        (f64::from(settings.preview_corner_radius) * scale).min(half_width.min(half_height));
    let thickness = f64::from(settings.preview_border_thickness) * scale;
    let accent_color = if settings.use_system_accent {
        system_accent.unwrap_or(settings.accent_color)
    } else {
        settings.accent_color
    };
    let gradient_color = if settings.use_gradient {
        settings.gradient_color
    } else {
        accent_color
    };
    let accent = rgb(accent_color);
    let gradient = rgb(gradient_color);
    let tint_alpha = if backdrop.is_some() {
        f64::from(settings.preview_opacity)
    } else {
        HUD_ALPHA
    };
    let interior_depth = EDGE_PX.max(thickness) + 0.5;
    let interior_lut = if !settings.use_gradient && backdrop.is_some() {
        let mut channels = [[0u32; 256]; 3];
        for channel in 0..3 {
            for (source, output) in channels[channel].iter_mut().enumerate() {
                let mut background = [0.0; 4];
                background[channel] = source as f64;
                background[3] = 255.0;
                let hud = src_over(background, HUD_COLOR, tint_alpha);
                let washed = src_over(hud, accent, ACCENT_WASH_ALPHA);
                *output = washed[channel].round().clamp(0.0, 255.0) as u32;
            }
        }
        Some(channels)
    } else {
        None
    };
    let plain_interior = (!settings.use_gradient && backdrop.is_none()).then(|| {
        let hud = src_over([0.0; 4], HUD_COLOR, tint_alpha);
        pack_premultiplied(src_over(hud, accent, ACCENT_WASH_ALPHA))
    });

    let mut pixels = vec![0u32; pixel_count as usize];
    let left = (i64::from(frame_x) - 1).clamp(0, i64::from(width));
    let top = (i64::from(frame_y) - 1).clamp(0, i64::from(height));
    let right = (i64::from(frame_x) + i64::from(frame_w) + 1).clamp(0, i64::from(width));
    let bottom = (i64::from(frame_y) + i64::from(frame_h) + 1).clamp(0, i64::from(height));

    for y in top..bottom {
        for x in left..right {
            let local_x = (x - i64::from(frame_x)) as f64;
            let local_y = (y - i64::from(frame_y)) as f64;
            let dx = local_x + 0.5 - half_width;
            let dy = local_y + 0.5 - half_height;
            let distance = rounded_box_distance(dx, dy, half_width, half_height, radius);
            let plate = shape_coverage(distance);
            if plate == 0.0 {
                continue;
            }
            let wash = if settings.use_gradient {
                let position = if frame_w <= 1 {
                    0.5
                } else {
                    (local_x / f64::from(frame_w - 1)).clamp(0.0, 1.0)
                };
                lerp_rgb(accent, gradient, position)
            } else {
                accent
            };
            let index = (y as usize) * (width as usize) + (x as usize);
            if distance <= -interior_depth {
                if let (Some(channels), Some(backdrop)) = (&interior_lut, backdrop) {
                    let source = backdrop[index];
                    pixels[index] = 0xff00_0000
                        | (channels[0][((source >> 16) & 0xff) as usize] << 16)
                        | (channels[1][((source >> 8) & 0xff) as usize] << 8)
                        | channels[2][(source & 0xff) as usize];
                    continue;
                }
                if let Some(pixel) = plain_interior {
                    pixels[index] = pixel;
                    continue;
                }
            }
            let background = backdrop.map_or([0.0; 4], |pixels| {
                let source = pixels[index];
                [
                    f64::from((source >> 16) & 0xff),
                    f64::from((source >> 8) & 0xff),
                    f64::from(source & 0xff),
                    255.0,
                ]
            });
            let mut pixel = src_over(background, HUD_COLOR, tint_alpha * plate);
            pixel = src_over(pixel, wash, ACCENT_WASH_ALPHA * plate);
            pixel = src_over(
                pixel,
                EDGE_COLOR,
                EDGE_ALPHA * band_coverage(distance, EDGE_PX),
            );
            if thickness > 0.0 {
                pixel = src_over(
                    pixel,
                    wash,
                    ACCENT_EDGE_ALPHA * band_coverage(distance, thickness),
                );
            }
            pixels[index] = pack_premultiplied(pixel);
        }
    }

    Ok(PreviewBitmap {
        width,
        height,
        pixels,
    })
}

/// Blur a captured screen rectangle in memory. One sample per 8 px keeps large previews quick;
/// two small box passes and bilinear reconstruction avoid visible sampling blocks.
pub fn blur_backdrop(pixels: &[u32], width: i32, height: i32) -> Result<Vec<u32>, String> {
    let count = usize::try_from(width).ok().and_then(|width| {
        usize::try_from(height)
            .ok()
            .and_then(|height| width.checked_mul(height))
    });
    if count.is_none_or(|count| count == 0 || pixels.len() != count) {
        return Err("invalid preview backdrop".into());
    }
    const STEP: usize = 8;
    const RADIUS: usize = 3;
    let width = width as usize;
    let height = height as usize;
    let small_width = width.div_ceil(STEP);
    let small_height = height.div_ceil(STEP);
    let mut small = vec![[0.0f32; 3]; small_width * small_height];
    for sy in 0..small_height {
        for sx in 0..small_width {
            let mut sum = [0u32; 3];
            let mut count = 0u32;
            for y in sy * STEP..((sy + 1) * STEP).min(height) {
                for x in sx * STEP..((sx + 1) * STEP).min(width) {
                    let pixel = pixels[y * width + x];
                    sum[0] += (pixel >> 16) & 0xff;
                    sum[1] += (pixel >> 8) & 0xff;
                    sum[2] += pixel & 0xff;
                    count += 1;
                }
            }
            small[sy * small_width + sx] = sum.map(|value| value as f32 / count as f32);
        }
    }
    let mut horizontal = vec![[0.0f32; 3]; small.len()];
    for y in 0..small_height {
        for x in 0..small_width {
            let start = x.saturating_sub(RADIUS);
            let end = (x + RADIUS + 1).min(small_width);
            let mut sum = [0.0f32; 3];
            for sample in &small[y * small_width + start..y * small_width + end] {
                for channel in 0..3 {
                    sum[channel] += sample[channel];
                }
            }
            horizontal[y * small_width + x] = sum.map(|value| value / (end - start) as f32);
        }
    }
    for y in 0..small_height {
        for x in 0..small_width {
            let start = y.saturating_sub(RADIUS);
            let end = (y + RADIUS + 1).min(small_height);
            let mut sum = [0.0f32; 3];
            for row in start..end {
                let sample = horizontal[row * small_width + x];
                for channel in 0..3 {
                    sum[channel] += sample[channel];
                }
            }
            small[y * small_width + x] = sum.map(|value| value / (end - start) as f32);
        }
    }
    let mut blurred = vec![0u32; pixels.len()];
    for y in 0..height {
        let fy = ((y as f32 + 0.5) / STEP as f32 - 0.5).max(0.0);
        let y0 = (fy as usize).min(small_height - 1);
        let y1 = (y0 + 1).min(small_height - 1);
        let ty = if y0 == y1 { 0.0 } else { fy - y0 as f32 };
        for x in 0..width {
            let fx = ((x as f32 + 0.5) / STEP as f32 - 0.5).max(0.0);
            let x0 = (fx as usize).min(small_width - 1);
            let x1 = (x0 + 1).min(small_width - 1);
            let tx = if x0 == x1 { 0.0 } else { fx - x0 as f32 };
            let mut color = 0xff00_0000;
            for (channel, top_left) in small[y0 * small_width + x0].iter().enumerate() {
                let top = top_left * (1.0 - tx) + small[y0 * small_width + x1][channel] * tx;
                let bottom = small[y1 * small_width + x0][channel] * (1.0 - tx)
                    + small[y1 * small_width + x1][channel] * tx;
                color |= ((top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0) as u32)
                    << (16 - channel * 8);
            }
            blurred[y * width + x] = color;
        }
    }
    Ok(blurred)
}

fn shape_coverage(distance: f64) -> f64 {
    (0.5 - distance).clamp(0.0, 1.0)
}

fn band_coverage(distance: f64, width: f64) -> f64 {
    if width <= 0.0 {
        return 0.0;
    }
    (shape_coverage(distance) - shape_coverage(distance + width)).clamp(0.0, 1.0)
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

fn rgb(color: u32) -> [f64; 3] {
    let color = color & 0x00ff_ffff;
    [
        f64::from((color >> 16) & 0xff),
        f64::from((color >> 8) & 0xff),
        f64::from(color & 0xff),
    ]
}

fn lerp_rgb(first: [f64; 3], second: [f64; 3], position: f64) -> [f64; 3] {
    [
        first[0] + (second[0] - first[0]) * position,
        first[1] + (second[1] - first[1]) * position,
        first[2] + (second[2] - first[2]) * position,
    ]
}

/// `dst` and the result are premultiplied RGBA in 0..=255. `straight` is straight RGB.
fn src_over(dst: [f64; 4], straight: [f64; 3], alpha: f64) -> [f64; 4] {
    if alpha <= 0.0 {
        return dst;
    }
    let inv = 1.0 - alpha / 255.0;
    let scale = alpha / 255.0;
    [
        straight[0] * scale + dst[0] * inv,
        straight[1] * scale + dst[1] * inv,
        straight[2] * scale + dst[2] * inv,
        alpha + dst[3] * inv,
    ]
}

fn pack_premultiplied(pixel: [f64; 4]) -> u32 {
    let channel = |value: f64| value.round().clamp(0.0, 255.0) as u32;
    let alpha = channel(pixel[3]);
    let red = channel(pixel[0]).min(alpha);
    let green = channel(pixel[1]).min(alpha);
    let blue = channel(pixel[2]).min(alpha);
    (alpha << 24) | (red << 16) | (green << 8) | blue
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(bitmap: &PreviewBitmap, x: i32, y: i32) -> u32 {
        bitmap.pixels[(y * bitmap.width + x) as usize]
    }

    #[test]
    fn renders_neutral_hud_with_an_accent_border() {
        let settings = Settings {
            preview_opacity: 128,
            preview_border_thickness: 4,
            ..Default::default()
        };
        let plate = render_bitmap(&settings, Action::LeftHalf, (40, 30), 96).unwrap();
        assert_eq!((plate.width, plate.height), (40, 30));
        assert_eq!(at(&plate, 20, 15), 0xff35_373a);
        assert_eq!(at(&plate, 1, 15) >> 24, 255, "accent border is opaque");
        assert_eq!(at(&plate, 0, 0) >> 24, 0, "outside the rounded corner");

        let screen = render_screen_preview(
            &settings,
            Action::LeftHalf,
            (80, 60),
            (10, 8, 40, 30),
            96,
            None,
        )
        .unwrap();
        assert_eq!((screen.width, screen.height), (80, 60));
        assert_eq!(at(&screen, 0, 0), 0, "outside the plate");
        assert_eq!(at(&screen, 9, 23), 0, "outside the plate");
        assert_eq!(at(&screen, 70, 50), 0, "outside the plate");
        assert_eq!(at(&screen, 30, 23), at(&plate, 20, 15));
        assert_eq!(at(&screen, 11, 23), at(&plate, 1, 15));

        let outline = Settings {
            preview_opacity: 40,
            preview_border_thickness: 0,
            preview_corner_radius: 0,
            ..Default::default()
        };
        let outlined = render_bitmap(&outline, Action::LeftHalf, (24, 16), 96).unwrap();
        assert_eq!(at(&outlined, 12, 8), at(&plate, 20, 15));
        assert_ne!(at(&outlined, 0, 8), at(&outlined, 12, 8));
    }

    #[test]
    fn gradient_interpolates_and_dpi_scales_the_bitmap_and_border() {
        let settings = Settings {
            use_gradient: true,
            accent_color: 0x000000,
            gradient_color: 0xffffff,
            preview_border_thickness: 2,
            preview_opacity: 128,
            preview_corner_radius: 0,
            ..Default::default()
        };
        let bitmap = render_bitmap(&settings, Action::RightHalf, (30, 18), 144).unwrap();
        assert_eq!((bitmap.width, bitmap.height), (30, 18));
        assert!(
            (at(&bitmap, 10, 9) & 0x00ff_ffff) < (at(&bitmap, 20, 9) & 0x00ff_ffff),
            "gradient changes color across the interior"
        );
        assert_eq!(at(&bitmap, 2, 9) >> 24, 255, "DPI scales the accent border");
        assert_eq!(at(&bitmap, 10, 9) >> 24, 255);
    }

    #[test]
    fn window_opacity_does_not_change_the_cached_material_bitmap() {
        let settings = Settings {
            preview_opacity: 200,
            padding: -1,
            ..Default::default()
        };
        let bitmap = render_bitmap(&settings, Action::LeftHalf, (20, 20), 96).unwrap();
        assert_eq!(at(&bitmap, 10, 10), 0xff35_373a);

        let clear = Settings {
            preview_opacity: 0,
            padding: -1,
            ..Default::default()
        };
        let clear = render_bitmap(&clear, Action::LeftHalf, (20, 20), 96).unwrap();
        assert_eq!(at(&clear, 10, 10), at(&bitmap, 10, 10));

        let solid = Settings {
            preview_opacity: 255,
            padding: -1,
            ..Default::default()
        };
        let solid = render_bitmap(&solid, Action::LeftHalf, (20, 20), 96).unwrap();
        assert_eq!(at(&solid, 10, 10), at(&bitmap, 10, 10));
    }

    #[test]
    fn captured_backdrop_blurs_across_edges_and_stays_inside_the_plate() {
        let mut captured = vec![0xff00_00ff; 64 * 24];
        for row in captured.chunks_mut(64) {
            row[..32].fill(0xffff_0000);
        }
        let blurred = blur_backdrop(&captured, 64, 24).unwrap();
        let middle = blurred[12 * 64 + 31];
        assert!((middle >> 16) & 0xff > 20);
        assert!(middle & 0xff > 20);

        let settings = Settings {
            preview_opacity: 170,
            ..Default::default()
        };
        let plate = render_bitmap_with_backdrop(
            &settings,
            Action::RightHalf,
            (64, 24),
            96,
            None,
            Some(&blurred),
        )
        .unwrap();
        assert_eq!(at(&plate, 0, 0), 0);
        assert_eq!(at(&plate, 31, 12) >> 24, 255);
        assert!(
            render_bitmap_with_backdrop(
                &settings,
                Action::RightHalf,
                (64, 24),
                96,
                None,
                Some(&blurred[..10]),
            )
            .is_err()
        );
    }

    #[test]
    fn cached_interior_matches_full_backdrop_composition() {
        let source = 0xff12_86e4;
        let backdrop = vec![source; 32 * 32];
        for opacity in [0, 42, 170, 255] {
            let settings = Settings {
                preview_opacity: opacity,
                ..Default::default()
            };
            let bitmap = render_bitmap_with_backdrop(
                &settings,
                Action::RightHalf,
                (32, 32),
                96,
                None,
                Some(&backdrop),
            )
            .unwrap();
            let background = [18.0, 134.0, 228.0, 255.0];
            let hud = src_over(background, HUD_COLOR, f64::from(opacity));
            let expected =
                pack_premultiplied(src_over(hud, rgb(settings.accent_color), ACCENT_WASH_ALPHA));
            assert_eq!(at(&bitmap, 16, 16), expected, "opacity {opacity}");
        }
    }
}
