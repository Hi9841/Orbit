use orbit::geometry::Action;
use orbit::settings::Settings;

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
#[cfg(test)]
pub fn render_bitmap(
    settings: &Settings,
    selected: Option<Action>,
    dpi: u32,
) -> Result<RadialBitmap, String> {
    render_bitmap_with_colors(settings, selected, dpi, None)
}

/// Render with an optional OS-resolved accent color while remaining independent of the desktop.
#[cfg(test)]
pub fn render_bitmap_with_colors(
    settings: &Settings,
    selected: Option<Action>,
    dpi: u32,
    system_accent: Option<u32>,
) -> Result<RadialBitmap, String> {
    let mut selected_slots = [false; 8];
    if let Some(action) = selected {
        for (slot, candidate) in settings.radial_actions.iter().enumerate() {
            selected_slots[slot] = *candidate == action;
        }
    }
    render(settings, selected, selected_slots, dpi, system_accent)
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
    render(settings, selected, selected_slots, dpi, None)
}

/// Draw exactly one menu direction. Repeated actions in different directions stay distinct.
pub fn compose_radial(
    selected_action: Option<Action>,
    selected_sector: Option<usize>,
    settings: &Settings,
    dpi: u32,
) -> Result<RadialBitmap, String> {
    if selected_sector.is_some_and(|sector| sector >= settings.radial_actions.len()) {
        return Err("selected radial sector is outside the eight menu directions".into());
    }
    let mut selected_slots = [false; 8];
    if let Some(sector) = selected_sector {
        selected_slots[sector] = true;
    }
    render(
        settings,
        selected_action,
        selected_slots,
        dpi,
        runtime_system_accent(settings),
    )
}

fn render(
    settings: &Settings,
    _selected_action: Option<Action>,
    selected_slots: [bool; 8],
    dpi: u32,
    system_accent: Option<u32>,
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
    let accent = (if settings.use_system_accent {
        system_accent.unwrap_or(settings.accent_color)
    } else {
        settings.accent_color
    }) & 0x00ff_ffff;
    let gradient = if settings.use_gradient {
        settings.gradient_color & 0x00ff_ffff
    } else {
        accent
    };
    let pixel_count = usize::try_from(size)
        .ok()
        .and_then(|value| value.checked_mul(value))
        .ok_or_else(|| "radial menu is too large to render".to_string())?;
    let mut pixels = vec![0u32; pixel_count];
    let stroke_half = thickness / 2.0;
    let centerline_half = outer - stroke_half;
    let centerline_corner = (outer_corner - stroke_half).max(0.0);
    let straight = centerline_half - centerline_corner;
    let quarter_length = 2.0 * straight + std::f64::consts::FRAC_PI_2 * centerline_corner;
    let perimeter = 4.0 * quarter_length;
    let selected_positions: Vec<f64> = selected_slots
        .iter()
        .enumerate()
        .filter(|(_, selected)| **selected)
        .map(|(slot, _)| slot as f64 * perimeter / 8.0)
        .collect();

    for y in 0..size {
        for x in 0..size {
            let dx = f64::from(x) + 0.5 - center;
            let dy = f64::from(y) + 0.5 - center;
            let outer_distance = rounded_box_distance(dx, dy, outer, outer, outer_corner);
            let inner_distance = rounded_box_distance(dx, dy, inner, inner, inner_corner);
            // Intersect rounded outer coverage with the inverse rounded inner shape. Radius 0
            // produces a square-corner ring; radius equal to half the diameter produces a circle.
            let ring_coverage = ((-outer_distance).min(inner_distance) + 0.5).clamp(0.0, 1.0);
            // A restrained light edge separates the dark HUD ring from its background.
            let edge_depth = (-outer_distance).min(inner_distance);
            let edge_strength = (1.0 - edge_depth / (1.5 * scale)).clamp(0.0, 1.0);
            let mut color = interpolate_rgb(0x354758, 0x718496, edge_strength * 0.7);
            let alpha = ring_coverage * 255.0;
            if ring_coverage > 0.0 && !selected_positions.is_empty() {
                let position = rounded_square_path_position(
                    dx,
                    dy,
                    straight,
                    centerline_corner,
                    quarter_length,
                );
                let normal = (stroke_half - edge_depth).clamp(0.0, stroke_half);
                let rounded_end = (stroke_half * stroke_half - normal * normal)
                    .max(0.0)
                    .sqrt();
                let arc_half = perimeter / 16.0;
                let mut cap_coverage = 0.0f64;
                let mut cap_fade = 0.0f64;
                for center_position in &selected_positions {
                    let delta = (position - center_position).abs();
                    let along = delta.min(perimeter - delta);
                    let coverage =
                        (arc_half - stroke_half + rounded_end - along + 0.5).clamp(0.0, 1.0);
                    if coverage > cap_coverage {
                        cap_coverage = coverage;
                        cap_fade = (1.0 - along / arc_half).clamp(0.0, 1.0);
                    }
                }
                let cap_color = if settings.use_gradient {
                    interpolate_rgb(accent, gradient, cap_fade)
                } else {
                    channels(accent)
                };
                for (value, cap_value) in color.iter_mut().zip(cap_color) {
                    *value += (cap_value - *value) * cap_coverage;
                }
                // Loop layers the ring border over the cap. This keeps a light cap
                // defined against a pale window without making the whole ring bright.
                for (value, rim_value) in color.iter_mut().zip(channels(0x777b7d)) {
                    *value += (rim_value - *value) * edge_strength * cap_coverage * 0.7;
                }
            }

            pixels[(y * size + x) as usize] = premultiplied_bgra(color, alpha);
        }
    }

    Ok(RadialBitmap { size, pixels })
}

fn rounded_square_path_position(
    x: f64,
    y: f64,
    straight: f64,
    corner: f64,
    quarter_length: f64,
) -> f64 {
    let x_abs = x.abs();
    let y_abs = y.abs();
    let first_quadrant = if corner <= 0.0 {
        if x_abs >= y_abs {
            y_abs
        } else {
            2.0 * straight - x_abs
        }
    } else if y_abs <= straight && (x_abs >= straight || x_abs >= y_abs) {
        y_abs
    } else if x_abs <= straight && (y_abs >= straight || y_abs > x_abs) {
        straight + std::f64::consts::FRAC_PI_2 * corner + straight - x_abs
    } else {
        straight
            + corner
                * (y_abs - straight)
                    .atan2(x_abs - straight)
                    .clamp(0.0, std::f64::consts::FRAC_PI_2)
    };
    match (x >= 0.0, y >= 0.0) {
        (true, true) => first_quadrant,
        (false, true) => 2.0 * quarter_length - first_quadrant,
        (false, false) => 2.0 * quarter_length + first_quadrant,
        (true, false) => 4.0 * quarter_length - first_quadrant,
    }
}

fn runtime_system_accent(settings: &Settings) -> Option<u32> {
    if !settings.use_system_accent {
        return None;
    }
    #[cfg(not(test))]
    {
        crate::platform::system_accent_rgb()
    }
    #[cfg(test)]
    {
        None
    }
}

fn rounded_box_distance(x: f64, y: f64, half_width: f64, half_height: f64, radius: f64) -> f64 {
    let radius = radius.min(half_width).min(half_height);
    let qx = x.abs() - half_width + radius;
    let qy = y.abs() - half_height + radius;
    if qx > 0.0 && qy > 0.0 {
        qx.hypot(qy) - radius
    } else {
        qx.max(qy) - radius
    }
}

fn premultiplied_bgra(color: [f64; 3], alpha: f64) -> u32 {
    let channel = |value: f64| (value * alpha / 255.0).round() as u32;
    let alpha = alpha.round() as u32;
    (alpha << 24) | (channel(color[0]) << 16) | (channel(color[1]) << 8) | channel(color[2])
}

fn channels(color: u32) -> [f64; 3] {
    [
        f64::from((color >> 16) & 0xff),
        f64::from((color >> 8) & 0xff),
        f64::from(color & 0xff),
    ]
}

fn interpolate_rgb(first: u32, second: u32, position: f64) -> [f64; 3] {
    let t = position.clamp(0.0, 1.0);
    let channel = |shift: u32| {
        let a = f64::from((first >> shift) & 0xffu32);
        let b = f64::from((second >> shift) & 0xffu32);
        a + (b - a) * t
    };
    [channel(16), channel(8), channel(0)]
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
        assert_eq!(
            at(&bitmap, 130, 90) >> 24,
            255,
            "the bitmap ring is fully covered"
        );
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
    fn selected_sector_is_a_brighter_cap_and_the_center_stays_open() {
        let settings = Settings::default();
        let bitmap = render_bitmap_for_sector(&settings, Some(0), 96).unwrap();
        assert_eq!(at(&bitmap, 90, 90) >> 24, 0);
        assert_eq!(at(&bitmap, 130, 90) >> 24, 255);
        assert_eq!(at(&bitmap, 50, 90) >> 24, 255);
        assert!(
            (at(&bitmap, 130, 90) & 0xff) > (at(&bitmap, 50, 90) & 0xff),
            "the aimed arc uses the accent, the rest of the ring stays neutral"
        );
        assert!(render_bitmap_for_sector(&settings, Some(8), 96).is_err());
    }

    #[test]
    fn pale_cap_retains_a_visible_outer_edge_on_a_light_window() {
        let bitmap = render_bitmap_for_sector(&Settings::default(), Some(0), 96).unwrap();
        let cap_middle = (at(&bitmap, 130, 90) >> 16) & 0xff;
        let cap_edge = (at(&bitmap, 139, 90) >> 16) & 0xff;
        assert!(cap_middle > 220);
        assert!(cap_edge < cap_middle - 25);
    }

    #[test]
    fn direction_cap_is_one_short_arc_on_the_stroke() {
        let settings = Settings {
            radial_thickness: 10,
            ..Settings::default()
        };
        let bitmap = render_bitmap_for_sector(&settings, Some(0), 96).unwrap();
        let center = f64::from(bitmap.size) / 2.0;
        let mut radii = Vec::new();
        let mut angles = Vec::new();
        for y in 0..bitmap.size {
            for x in 0..bitmap.size {
                let pixel = at(&bitmap, x, y);
                let red = (pixel >> 16) & 0xff;
                let green = (pixel >> 8) & 0xff;
                let blue = pixel & 0xff;
                let aimed = pixel >> 24 > 200 && red > 220 && green > 220 && blue > 220;
                if aimed {
                    let dx = f64::from(x) + 0.5 - center;
                    let dy = f64::from(y) + 0.5 - center;
                    radii.push(dx.hypot(dy));
                    angles.push(dy.atan2(dx));
                }
            }
        }
        assert!(!radii.is_empty());
        let min_radius = radii.iter().copied().fold(f64::MAX, f64::min);
        let max_radius = radii.iter().copied().fold(0.0, f64::max);
        assert!(
            min_radius > 20.0,
            "the cap must stay on the stroke, not fill a pie"
        );
        assert!(
            max_radius - min_radius < 16.0,
            "the cap must stay thin, span {}",
            max_radius - min_radius
        );
        let min_angle = angles.iter().copied().fold(f64::MAX, f64::min);
        let max_angle = angles.iter().copied().fold(f64::MIN, f64::max);
        assert!(max_angle - min_angle < 0.9, "the cap must be one short arc");
    }

    #[test]
    fn system_accent_tints_the_direction_cap() {
        let settings = Settings {
            use_system_accent: true,
            accent_color: 0x000000,
            radial_actions: [Action::RightHalf; 8],
            ..Settings::default()
        };
        let bitmap =
            render_bitmap_with_colors(&settings, Some(Action::RightHalf), 96, Some(0xff0000))
                .unwrap();
        let cap = at(&bitmap, 130, 90);
        assert_eq!(cap & 0x00ff_0000, 0x00ff_0000);
        assert!(cap >> 24 > 100);
    }
}
