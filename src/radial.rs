use orbit::geometry::Action;
use orbit::settings::Settings;

const CANVAS_GUTTER_PT: u32 = 80;
/// Neutral glass. Equal channels, no blue cast.
/// Prism's dark shell: oklch(0.17 0.009 268) over oklch(0.152 0.008 268).
const RING_OUTER: u32 = 0x1C1D22;
const RING_INNER: u32 = 0x16171C;
/// Quiet hairline, the same 8% white edge as Prism's shell.
const RING_LINE: u32 = 0xFFFFFF;
/// Selected arc. Prism foreground, not a blown-out white pill.
const RING_CAP: u32 = 0xE8E8EC;

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
/// The live ring uses `paint_stamp` so the cap can sit between those directions.
#[allow(dead_code)]
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
    let bitmap = render(
        settings,
        selected_action,
        selected_slots,
        dpi,
        runtime_system_accent(settings),
    )?;
    let expected = (bitmap.size as usize).saturating_mul(bitmap.size as usize);
    if bitmap.pixels.len() != expected {
        return Err("radial bitmap size does not match its pixels".into());
    }
    Ok(bitmap)
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

    let draw = RingDraw {
        outer,
        inner,
        outer_corner,
        inner_corner,
        thickness,
        scale,
        stroke_half,
        straight,
        centerline_corner,
        perimeter,
        accent,
        gradient,
        use_gradient: settings.use_gradient,
    };
    let mut aims = [0.0; 8];
    let mut aim_count = 0;
    for (slot, selected) in selected_slots.iter().enumerate() {
        if *selected {
            aims[aim_count] = slot as f64 * perimeter / 8.0;
            aim_count += 1;
        }
    }
    for y in 0..size {
        for x in 0..size {
            pixels[(y * size + x) as usize] =
                draw.supersample(f64::from(x), f64::from(y), center, &aims[..aim_count]);
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

#[derive(Clone)]
pub struct RingSample {
    pub index: u32,
    pub position: f32,
    #[allow(dead_code)]
    pub edge_strength: f32,
    #[allow(dead_code)]
    pub alpha: f32,
    #[allow(dead_code)]
    pub color: [f32; 3],
}

/// Idle ring plus the samples needed to slide the direction cap.
#[derive(Clone)]
pub struct RadialStamp {
    pub size: i32,
    pub idle: Vec<u32>,
    pub perimeter: f64,
    pub samples: Vec<RingSample>,
    pub accent: u32,
    pub gradient: u32,
    pub use_gradient: bool,
    pub straight: f64,
    pub centerline_corner: f64,
    pub centerline_half: f64,
    pub stroke_half: f64,
    pub scale: f64,
}

struct RadialMetrics {
    size: i32,
    scale: f64,
    center: f64,
    outer: f64,
    inner: f64,
    outer_corner: f64,
    inner_corner: f64,
    accent: u32,
    gradient: u32,
    stroke_half: f64,
    straight: f64,
    centerline_corner: f64,
    centerline_half: f64,
    perimeter: f64,
}

fn metrics(
    settings: &Settings,
    dpi: u32,
    system_accent: Option<u32>,
) -> Result<RadialMetrics, String> {
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
    let stroke_half = thickness / 2.0;
    let centerline_half = outer - stroke_half;
    let centerline_corner = (outer_corner - stroke_half).max(0.0);
    let straight = centerline_half - centerline_corner;
    let quarter_length = 2.0 * straight + std::f64::consts::FRAC_PI_2 * centerline_corner;
    Ok(RadialMetrics {
        size,
        scale,
        center,
        outer,
        inner,
        outer_corner,
        inner_corner,
        accent,
        gradient,
        stroke_half,
        straight,
        centerline_corner,
        centerline_half,
        perimeter: 4.0 * quarter_length,
    })
}

/// Build the idle ring once. Later frames only recolor samples near the cursor.
pub fn build_stamp(settings: &Settings, dpi: u32) -> Result<RadialStamp, String> {
    let metrics = metrics(settings, dpi, runtime_system_accent(settings))?;
    let pixel_count = usize::try_from(metrics.size)
        .ok()
        .and_then(|value| value.checked_mul(value))
        .ok_or_else(|| "radial menu is too large to render".to_string())?;
    let mut idle = vec![0u32; pixel_count];
    let mut samples = Vec::new();
    let draw = RingDraw {
        outer: metrics.outer,
        inner: metrics.inner,
        outer_corner: metrics.outer_corner,
        inner_corner: metrics.inner_corner,
        thickness: metrics.outer - metrics.inner,
        scale: metrics.scale,
        stroke_half: metrics.stroke_half,
        straight: metrics.straight,
        centerline_corner: metrics.centerline_corner,
        perimeter: metrics.perimeter,
        accent: metrics.accent,
        gradient: metrics.gradient,
        use_gradient: settings.use_gradient,
    };
    for y in 0..metrics.size {
        for x in 0..metrics.size {
            let dx = f64::from(x) + 0.5 - metrics.center;
            let dy = f64::from(y) + 0.5 - metrics.center;
            if dx.abs().max(dy.abs()) > metrics.outer + 1.5 {
                continue;
            }
            let index = (y * metrics.size + x) as usize;
            idle[index] = draw.supersample(f64::from(x), f64::from(y), metrics.center, &[]);
            if idle[index] >> 24 == 0 {
                continue;
            }
            let position = rounded_square_path_position(
                dx,
                dy,
                metrics.straight,
                metrics.centerline_corner,
                metrics.perimeter / 4.0,
            );
            samples.push(RingSample {
                index: index as u32,
                position: position as f32,
                edge_strength: 0.0,
                alpha: (idle[index] >> 24) as f32,
                color: [0.0, 0.0, 0.0],
            });
        }
    }
    Ok(RadialStamp {
        size: metrics.size,
        idle,
        perimeter: metrics.perimeter,
        samples,
        accent: metrics.accent,
        gradient: metrics.gradient,
        use_gradient: settings.use_gradient,
        straight: metrics.straight,
        centerline_corner: metrics.centerline_corner,
        centerline_half: metrics.centerline_half,
        stroke_half: metrics.stroke_half,
        scale: metrics.scale,
    })
}

fn ring_draw_from_stamp(stamp: &RadialStamp) -> RingDraw {
    let thickness = stamp.stroke_half * 2.0;
    let outer = stamp.centerline_half + stamp.stroke_half;
    let inner = (stamp.centerline_half - stamp.stroke_half).max(0.0);
    RingDraw {
        outer,
        inner,
        outer_corner: stamp.centerline_corner + stamp.stroke_half,
        inner_corner: stamp.centerline_corner.max(0.0).min(inner),
        thickness,
        scale: stamp.scale,
        stroke_half: stamp.stroke_half,
        straight: stamp.straight,
        centerline_corner: stamp.centerline_corner,
        perimeter: stamp.perimeter,
        accent: stamp.accent,
        gradient: stamp.gradient,
        use_gradient: stamp.use_gradient,
    }
}

fn position_at_angle(stamp: &RadialStamp, angle: f64) -> f64 {
    let (sin, cos) = angle.sin_cos();
    let mut lo = 0.0;
    let mut hi = stamp.centerline_half * 2.0 + 2.0;
    for _ in 0..24 {
        let mid = (lo + hi) * 0.5;
        let distance = rounded_box_distance(
            cos * mid,
            sin * mid,
            stamp.centerline_half,
            stamp.centerline_half,
            stamp.centerline_corner,
        );
        if distance > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let radius = (lo + hi) * 0.5;
    rounded_square_path_position(
        cos * radius,
        sin * radius,
        stamp.straight,
        stamp.centerline_corner,
        stamp.perimeter / 4.0,
    )
}

/// `angle` is `dy.atan2(dx)` with y growing downward. `None` leaves the ring idle.
pub fn paint_stamp(stamp: &RadialStamp, angle: Option<f64>) -> Vec<u32> {
    let mut pixels = stamp.idle.clone();
    let Some(angle) = angle else {
        return pixels;
    };
    let width = stamp.size as usize;
    if width == 0 || pixels.len() != width * width {
        return pixels;
    }
    let center_position = position_at_angle(stamp, angle);
    let arc_half = cap_half(stamp.perimeter);
    let window = arc_half + stamp.stroke_half + 4.0;
    let draw = ring_draw_from_stamp(stamp);
    let center = f64::from(stamp.size) / 2.0;
    let aim = [center_position];
    for sample in &stamp.samples {
        let delta = (f64::from(sample.position) - center_position).abs();
        let along = delta.min(stamp.perimeter - delta);
        if along > window {
            continue;
        }
        let x = f64::from(sample.index % width as u32);
        let y = f64::from(sample.index / width as u32);
        pixels[sample.index as usize] = draw.supersample(x, y, center, &aim);
    }
    pixels
}

struct RingShade {
    color: [f64; 3],
    coverage: f64,
    #[allow(dead_code)]
    edge_depth: f64,
}

struct RingDraw {
    outer: f64,
    inner: f64,
    outer_corner: f64,
    inner_corner: f64,
    thickness: f64,
    scale: f64,
    #[allow(dead_code)]
    stroke_half: f64,
    straight: f64,
    centerline_corner: f64,
    perimeter: f64,
    accent: u32,
    gradient: u32,
    use_gradient: bool,
}

impl RingDraw {
    /// 3x3 grid. Interior pixels stay fully solid; only the rim is averaged.
    fn supersample(&self, x: f64, y: f64, center: f64, aims: &[f64]) -> u32 {
        let offsets = [-0.375, -0.125, 0.125, 0.375];
        let mut alpha_sum = 0.0;
        let mut color_sum = [0.0, 0.0, 0.0];
        let samples = (offsets.len() * offsets.len()) as f64;
        for oy in offsets {
            for ox in offsets {
                let (color, alpha) = self.fragment(x + 0.5 + ox, y + 0.5 + oy, center, aims);
                alpha_sum += alpha;
                for (channel, value) in color_sum.iter_mut().zip(color) {
                    *channel += value * alpha;
                }
            }
        }
        let alpha = alpha_sum / samples;
        let color = if alpha_sum > 0.0 {
            [
                color_sum[0] / alpha_sum,
                color_sum[1] / alpha_sum,
                color_sum[2] / alpha_sum,
            ]
        } else {
            [0.0, 0.0, 0.0]
        };
        premultiplied_bgra(color, alpha)
    }

    fn fragment(&self, x: f64, y: f64, center: f64, aims: &[f64]) -> ([f64; 3], f64) {
        let dx = x - center;
        let dy = y - center;
        let circle = self.outer_corner >= self.outer - 0.5;
        let (outer_distance, inner_distance) = if circle {
            let radius = dx.hypot(dy);
            (radius - self.outer, radius - self.inner)
        } else {
            (
                rounded_box_distance(dx, dy, self.outer, self.outer, self.outer_corner),
                rounded_box_distance(dx, dy, self.inner, self.inner, self.inner_corner),
            )
        };
        let shade = shade_ring(outer_distance, inner_distance, self.thickness, self.scale);
        let mut color = shade.color;
        let sky = ((-dy) / self.outer.max(1.0)).clamp(0.0, 1.0);
        blend_toward(&mut color, channels(RING_LINE), sky * 0.05);
        if shade.coverage > 0.0 && !aims.is_empty() {
            let along = if circle {
                let angle = dy.atan2(dx);
                let mut best = f64::MAX;
                for center_position in aims {
                    let aim = *center_position / self.perimeter * std::f64::consts::TAU;
                    let mut delta = (angle - aim).abs();
                    if delta > std::f64::consts::PI {
                        delta = std::f64::consts::TAU - delta;
                    }
                    best = best.min(delta);
                }
                best
            } else {
                let quarter = self.perimeter / 4.0;
                let position = rounded_square_path_position(
                    dx,
                    dy,
                    self.straight,
                    self.centerline_corner,
                    quarter,
                );
                aims.iter()
                    .map(|center_position| {
                        circular_along(position, *center_position, self.perimeter)
                    })
                    .fold(f64::MAX, f64::min)
            };
            let half = if circle {
                0.42
            } else {
                cap_half(self.perimeter)
            };
            let cap_coverage = smoothstep((half - along) / (half * 0.78));
            let cap_fade = (1.0 - along / half.max(0.001)).clamp(0.0, 1.0);
            let cap_color = if self.use_gradient {
                interpolate_rgb(self.accent, self.gradient, cap_fade)
            } else {
                channels(RING_CAP)
            };
            // A lifted selection, like Prism's active row, kept inside the stroke.
            blend_toward(&mut color, cap_color, cap_coverage * 0.82);
        }
        (color, shade.coverage * 255.0)
    }
}

fn shade_ring(outer_distance: f64, inner_distance: f64, thickness: f64, scale: f64) -> RingShade {
    let edge = (-outer_distance).min(inner_distance);
    let coverage = smoothstep((edge + 1.35) / 2.7);
    let edge_depth = edge.max(0.0);
    let from_outer = (-outer_distance).max(0.0);
    let depth = (from_outer / thickness.max(1.0)).clamp(0.0, 1.0);
    let mut color = interpolate_rgb(RING_OUTER, RING_INNER, depth * 0.65);
    let hair = (1.0 - from_outer / (1.15 * scale.max(1.0))).clamp(0.0, 1.0);
    blend_toward(&mut color, channels(RING_LINE), hair * hair * 0.08);
    RingShade {
        color,
        coverage,
        edge_depth,
    }
}

fn circular_along(position: f64, center: f64, perimeter: f64) -> f64 {
    let delta = (position - center).abs();
    delta.min(perimeter - delta)
}

/// Half-length of the aim cap. About 0.32 rad on a circle, short of one sector.
fn cap_half(perimeter: f64) -> f64 {
    (perimeter * 0.32 / std::f64::consts::TAU).max(4.0)
}

fn smoothstep(value: f64) -> f64 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn blend_toward(color: &mut [f64; 3], target: [f64; 3], amount: f64) {
    let amount = amount.clamp(0.0, 1.0);
    for (value, target) in color.iter_mut().zip(target) {
        *value += (target - *value) * amount;
    }
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
    fn glass_cap_is_white_and_the_outside_stays_clear() {
        let bitmap = render_bitmap_for_sector(&Settings::default(), Some(0), 96).unwrap();
        let cap_middle = (at(&bitmap, 130, 90) >> 16) & 0xff;
        assert!(
            cap_middle > 150,
            "the aimed arc is the lifted shell selection"
        );
        assert!(at(&bitmap, 139, 90) >> 24 > 80, "the rim is still drawn");
        assert_eq!(
            at(&bitmap, 150, 90) >> 24,
            0,
            "outside the ring stays clear"
        );
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
                let aimed = pixel >> 24 > 200 && red > 150 && green > 150 && blue > 150;
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
    fn direction_cap_stays_neutral_glass_instead_of_a_blue_accent() {
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
        let red = (cap >> 16) & 0xff;
        let green = (cap >> 8) & 0xff;
        let blue = cap & 0xff;
        assert!(
            red > 150 && green > 150 && blue > 150,
            "cap {cap:#x} should be the lifted shell selection"
        );
        assert!(
            (red as i32 - blue as i32).abs() < 18,
            "cap must not pick up the accent hue"
        );
    }

    #[test]
    fn stamp_cap_follows_the_cursor_angle_instead_of_snapping_early() {
        let settings = Settings::default();
        let stamp = build_stamp(&settings, 96).unwrap();
        let east = paint_stamp(&stamp, Some(0.0));
        let nudged = paint_stamp(&stamp, Some(10.0_f64.to_radians()));
        let idle = paint_stamp(&stamp, None);
        let at = |pixels: &[u32], x: i32, y: i32| pixels[(y * stamp.size + x) as usize];
        assert!(
            (at(&east, 130, 90) & 0xff) > (at(&east, 50, 90) & 0xff),
            "angle 0 aims the cap east"
        );
        assert_ne!(east, nudged, "a 10 degree move must slide the cap");
        assert_eq!(idle, stamp.idle);
        for sector in [0usize, 2, 4, 6] {
            let mut angle = sector as f64 * std::f64::consts::FRAC_PI_4;
            if angle > std::f64::consts::PI {
                angle -= std::f64::consts::TAU;
            }
            let position = position_at_angle(&stamp, angle);
            let expected = sector as f64 * stamp.perimeter / 8.0;
            let delta = (position - expected)
                .abs()
                .min(stamp.perimeter - (position - expected).abs());
            assert!(
                delta < 1.5,
                "sector {sector} arc error {delta} (position {position}, expected {expected})"
            );
        }
    }
}
