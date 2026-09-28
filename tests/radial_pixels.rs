// Pure rendering only. This test never creates an HWND or draws on the desktop.
#[allow(dead_code)]
#[path = "../src/radial.rs"]
mod radial;

use orbit::settings::Settings;

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
    let Ok(output) = std::env::var("ORBIT_RENDER_OUTPUT") else {
        return;
    };
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
    std::fs::write(output, bmp).unwrap();
}
