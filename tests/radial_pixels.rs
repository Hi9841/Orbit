// Pure rendering only. This test never creates an HWND or draws on the desktop.
#[allow(dead_code)]
#[path = "../src/preview.rs"]
mod preview;
#[allow(dead_code)]
#[path = "../src/radial.rs"]
mod radial;

use orbit::{geometry::Action, settings::Settings};

#[test]
fn radial_pixels_are_premultiplied_and_can_be_inspected_offscreen() {
    let variants = [
        (Settings::default(), Some(0), 96),
        (
            Settings {
                radial_corner_radius: 12,
                ..Default::default()
            },
            Some(5),
            96,
        ),
        (
            Settings {
                radial_size: 120,
                radial_thickness: 18,
                accent_color: 0x9D75FF,
                ..Default::default()
            },
            Some(2),
            144,
        ),
    ];
    let rendered: Vec<_> = variants
        .iter()
        .map(|(settings, sector, dpi)| {
            radial::render_bitmap_for_sector(settings, *sector, *dpi).unwrap()
        })
        .collect();
    for bitmap in &rendered {
        assert_eq!(bitmap.pixels.len(), (bitmap.size * bitmap.size) as usize);
        for pixel in &bitmap.pixels {
            let alpha = pixel >> 24;
            assert!(
                pixel & 0xff <= alpha
                    && (pixel >> 8) & 0xff <= alpha
                    && (pixel >> 16) & 0xff <= alpha
            );
        }
    }
    if let Ok(output) = std::env::var("ORBIT_RENDER_OUTPUT") {
        write_bmp(&output, radial_contact_sheet(&rendered));
    }
}

#[test]
fn preview_pixels_preserve_alpha_gradient_and_can_be_inspected_offscreen() {
    let settings = [
        Settings::default(),
        Settings {
            preview_opacity: 128,
            preview_border_thickness: 5,
            preview_corner_radius: 18,
            use_gradient: true,
            accent_color: 0x204060,
            gradient_color: 0xe0c080,
            ..Default::default()
        },
    ];
    let rendered: Vec<_> = settings
        .iter()
        .map(|settings| {
            preview::render_bitmap(settings, Action::RightHalf, (320, 180), 144).unwrap()
        })
        .collect();

    for bitmap in &rendered {
        assert_eq!(bitmap.pixels.len(), (bitmap.width * bitmap.height) as usize);
        for pixel in &bitmap.pixels {
            let alpha = pixel >> 24;
            assert!(
                (pixel >> 16) & 0xff <= alpha
                    && (pixel >> 8) & 0xff <= alpha
                    && pixel & 0xff <= alpha
            );
        }
    }

    let at = |index: usize, x: i32, y: i32| {
        rendered[index].pixels[(y * rendered[index].width + x) as usize]
    };
    assert_eq!(
        at(0, 160, 90) >> 24,
        65,
        "default opacity is a raw alpha byte"
    );
    assert_eq!(at(0, 2, 90) >> 24, 255, "border remains opaque");
    assert_eq!(at(0, 0, 0) >> 24, 0, "rounded corners stay transparent");
    let center = at(0, 160, 90);
    assert_eq!((center >> 16) & 0xff, 26); // R=103 premultiplied by alpha 65
    assert_eq!((center >> 8) & 0xff, 49); // G=193
    assert_eq!(center & 0xff, 55); // B=214
    assert!((at(1, 2, 90) & 0x00ff_ffff) < (at(1, 317, 90) & 0x00ff_ffff));

    let system_accent = Settings {
        use_system_accent: true,
        accent_color: 0x67c1d6,
        preview_opacity: 255,
        preview_border_thickness: 0,
        preview_corner_radius: 0,
        ..Default::default()
    };
    let resolved = preview::render_bitmap_with_colors(
        &system_accent,
        Action::RightHalf,
        (20, 20),
        96,
        Some(0xff0000),
    )
    .unwrap();
    assert_eq!(resolved.pixels[10 * 20 + 10], 0xffff0000);

    if let Ok(output) = std::env::var("ORBIT_PREVIEW_RENDER_OUTPUT") {
        write_bmp(&output, preview_contact_sheet(&rendered));
    }
}

fn radial_contact_sheet(rendered: &[radial::RadialBitmap]) -> (i32, i32, Vec<u32>) {
    let width = rendered.iter().map(|b| b.size).sum::<i32>();
    let height = rendered.iter().map(|b| b.size).max().unwrap();
    let mut pixels = vec![0xff20242bu32; (width * height) as usize];
    let mut offset = 0;
    for bitmap in rendered {
        for y in 0..bitmap.size {
            for x in 0..bitmap.size {
                let src = bitmap.pixels[(y * bitmap.size + x) as usize];
                let alpha = (src >> 24) & 0xff;
                let background = 0x20242bu32;
                let mut composite = 0xff000000;
                for shift in [0, 8, 16] {
                    let channel = ((src >> shift) & 0xff)
                        + (((background >> shift) & 0xff) * (255 - alpha) / 255);
                    composite |= channel.min(255) << shift;
                }
                pixels[(y * width + offset + x) as usize] = composite;
            }
        }
        offset += bitmap.size;
    }
    (width, height, pixels)
}

fn preview_contact_sheet(rendered: &[preview::PreviewBitmap]) -> (i32, i32, Vec<u32>) {
    let width = rendered.iter().map(|b| b.width).sum::<i32>();
    let height = rendered.iter().map(|b| b.height).max().unwrap();
    let mut pixels = vec![0xff20242bu32; (width * height) as usize];
    let mut offset = 0;
    for bitmap in rendered {
        for y in 0..bitmap.height {
            for x in 0..bitmap.width {
                let src = bitmap.pixels[(y * bitmap.width + x) as usize];
                pixels[(y * width + offset + x) as usize] = composite_on_dark(src);
            }
        }
        offset += bitmap.width;
    }
    (width, height, pixels)
}

fn composite_on_dark(src: u32) -> u32 {
    let alpha = (src >> 24) & 0xff;
    let background = 0x20242bu32;
    let mut composite = 0xff000000;
    for shift in [0, 8, 16] {
        let channel =
            ((src >> shift) & 0xff) + (((background >> shift) & 0xff) * (255 - alpha) / 255);
        composite |= channel.min(255) << shift;
    }
    composite
}

fn write_bmp(path: &str, (width, height, pixels): (i32, i32, Vec<u32>)) {
    let length = 54 + pixels.len() * 4;
    let mut bmp = Vec::with_capacity(length);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(length as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&(-height).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&[0; 24]);
    for pixel in pixels {
        bmp.extend_from_slice(&pixel.to_le_bytes());
    }
    std::fs::write(path, bmp).unwrap();
}
