//! Images in tests: straight-RGBA bitmaps, PNG files, and the comparison render goldens use
//! (SSIM on luma, ADR 0004: ≥ 0.99 against a backend's own golden).
#![expect(
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    reason = "test helpers (a panic is a test failure); SSIM uses its paper's names; pixel counts are small"
)]

use std::path::Path;

/// One rendered bitmap.
pub struct Bitmap {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Straight RGBA8.
    pub rgba: Vec<u8>,
}

impl Bitmap {
    /// The pixels of the rectangle `[x0, y0, x1, y1)` (pixels, clipped to the bitmap), row by row.
    #[must_use]
    pub fn pixels_in(&self, [x0, y0, x1, y1]: [u32; 4]) -> Vec<[u8; 4]> {
        let mut out = Vec::new();
        for y in y0..y1.min(self.height) {
            for x in x0..x1.min(self.width) {
                let i = ((y * self.width + x) * 4) as usize;
                out.push([self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]);
            }
        }
        out
    }
}

/// The WCAG 2 contrast ratio of two opaque sRGB colours (alpha ignored), 1 to 21.
#[must_use]
pub fn contrast(a: [u8; 4], b: [u8; 4]) -> f64 {
    let luminance = |[r, g, b, _]: [u8; 4]| {
        let linear = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    };
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Writes `bitmap` to `path` as an RGBA PNG.
pub fn write_png(path: &Path, bitmap: &Bitmap) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), bitmap.width, bitmap.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    encoder.write_header().unwrap().write_image_data(&bitmap.rgba).unwrap();
}

/// Reads the 8-bit RGBA or RGB PNG at `path` (RGB made opaque RGBA).
pub fn read_png(path: &Path) -> Bitmap {
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    ));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    pixels.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::Rgb => pixels.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        other => panic!("{}: {other:?} PNG, not RGBA or RGB", path.display()),
    };
    Bitmap {
        width: info.width,
        height: info.height,
        rgba,
    }
}

/// Mean SSIM of the two bitmaps' luma over 8 × 8 windows (stride 4). 1.0 is identical.
pub fn ssim(a: &Bitmap, b: &Bitmap) -> f64 {
    assert_eq!((a.width, a.height), (b.width, b.height), "sizes differ");
    let luma = |bm: &Bitmap| -> Vec<f64> {
        bm.rgba
            .chunks(4)
            .map(|p| 0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]))
            .collect()
    };
    let (la, lb) = (luma(a), luma(b));
    let (w, h) = (a.width as usize, a.height as usize);
    let (c1, c2) = ((0.01 * 255.0f64).powi(2), (0.03 * 255.0f64).powi(2));
    let (mut total, mut windows) = (0.0, 0usize);
    let size = 8.min(w).min(h);
    let mut y = 0;
    while y + size <= h {
        let mut x = 0;
        while x + size <= w {
            let n = (size * size) as f64;
            let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for yy in y..y + size {
                for xx in x..x + size {
                    let (p, q) = (la[yy * w + xx], lb[yy * w + xx]);
                    sa += p;
                    sb += q;
                    saa += p * p;
                    sbb += q * q;
                    sab += p * q;
                }
            }
            let (ma, mb) = (sa / n, sb / n);
            let (va, vb, cov) = (saa / n - ma * ma, sbb / n - mb * mb, sab / n - ma * mb);
            total += ((2.0 * ma * mb + c1) * (2.0 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2));
            windows += 1;
            x += 4;
        }
        y += 4;
    }
    if windows == 0 { 1.0 } else { total / windows as f64 }
}
