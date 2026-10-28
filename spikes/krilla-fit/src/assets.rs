//! Inputs shared by every experiment: the font, a PNG with alpha (generated here) and a JPEG
//! (committed: `assets/photo.jpg`, made by `scripts/make-jpeg.sh` from a PNG this module writes,
//! because every pure-Rust JPEG *encoder* on crates.io carries the IJG licence, which is not on
//! our allow-list).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Everything a generator needs, as bytes (the generators themselves never touch the disk).
pub struct Assets {
    pub font: Vec<u8>,
    /// RGBA PNG, 192×192, with a radial alpha falloff.
    pub png_alpha: Vec<u8>,
    /// Baseline JPEG, 192×192, RGB.
    pub jpeg: Vec<u8>,
    pub jpeg_size: (u32, u32),
}

pub const IMAGE_SIDE: u32 = 192;

pub fn spike_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn load() -> Result<Assets> {
    let dir = spike_dir();
    let font_path = dir.join("fonts/NotoSans-Regular.ttf");
    let font = std::fs::read(&font_path)
        .with_context(|| format!("{} missing: run scripts/fetch-font.sh", font_path.display()))?;
    let jpeg =
        std::fs::read(dir.join("assets/photo.jpg")).context("assets/photo.jpg missing: run scripts/make-jpeg.sh")?;
    let jpeg_size = jpeg_dimensions(&jpeg)?;
    Ok(Assets {
        font,
        png_alpha: png_rgba(&alpha_pixels())?,
        jpeg,
        jpeg_size,
    })
}

/// A colour wheel-ish gradient whose alpha falls off towards the corners.
fn alpha_pixels() -> Vec<u8> {
    let n = IMAGE_SIDE;
    let mut px = Vec::with_capacity((n * n * 4) as usize);
    let c = f64::from(n) / 2.0;
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (f64::from(x) - c, f64::from(y) - c);
            let d = (dx * dx + dy * dy).sqrt() / c;
            let a = (1.0 - d).clamp(0.0, 1.0);
            px.extend_from_slice(&[(x * 255 / n) as u8, (y * 255 / n) as u8, 200, (a * 255.0).round() as u8]);
        }
    }
    px
}

/// An opaque pattern used as the JPEG source (stripes over a gradient).
pub fn photo_pixels() -> Vec<u8> {
    let n = IMAGE_SIDE;
    let mut px = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let stripe = if (x / 16 + y / 16) % 2 == 0 { 40 } else { 0 };
            px.extend_from_slice(&[(200 - y * 150 / n) as u8, (60 + stripe) as u8, (x * 255 / n) as u8, 255]);
        }
    }
    px
}

pub fn png_rgba(rgba: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, IMAGE_SIDE, IMAGE_SIDE);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header()?;
        w.write_image_data(rgba)?;
    }
    Ok(out)
}

/// Writes the JPEG source PNG (for `scripts/make-jpeg.sh`).
pub fn write_photo_source(path: &Path) -> Result<()> {
    std::fs::write(path, png_rgba(&photo_pixels())?)?;
    Ok(())
}

/// Width and height from the first SOFn marker (ITU-T T.81 §B.1.1.2–B.1.1.4).
///
/// Walks the marker segments: any number of 0xFF fill bytes may precede a marker code;
/// standalone markers (SOI, EOI, TEM, RST0–7) have no length field. Scanning stops at SOS,
/// since entropy-coded data follows (a SOF always comes before it).
pub fn jpeg_dimensions(data: &[u8]) -> Result<(u32, u32)> {
    let byte = |i: usize| {
        data.get(i)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("truncated JPEG at {i}"))
    };
    let mut i = 0;
    loop {
        if byte(i)? != 0xFF {
            bail!("expected a JPEG marker at {i}");
        }
        while byte(i)? == 0xFF {
            i += 1; // fill bytes
        }
        let marker = byte(i)?;
        i += 1;
        match marker {
            0xD8 | 0xD9 | 0x01 | 0xD0..=0xD7 => continue, // SOI, EOI, TEM, RSTn: no length
            0xDA => bail!("no SOF marker before SOS"),
            _ => {}
        }
        let len = usize::from(u16::from_be_bytes([byte(i)?, byte(i + 1)?]));
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            // Lf(2) P(1) Y(2) X(2)
            let h = u16::from_be_bytes([byte(i + 3)?, byte(i + 4)?]);
            let w = u16::from_be_bytes([byte(i + 5)?, byte(i + 6)?]);
            return Ok((u32::from(w), u32::from(h)));
        }
        if len < 2 {
            bail!("bad JPEG segment length at {i}");
        }
        i += len;
    }
}

/// The same text in every experiment: (page, lines).
pub const PAGES: [(&str, &[&str]); 3] = [
    (
        "Page 1: text, vectors and a link",
        &[
            "krilla-fit spike: the same three pages, generated three ways.",
            "Text is shaped with a single font and embedded as a subset.",
            "Visit https://example.com (this line is a link annotation).",
            "Glyphs: AVATAR fi fl ffi -- quotes \u{201c}curly\u{201d} and \u{e9}\u{e8}\u{fc}\u{df}.",
        ],
    ),
    (
        "Page 2: images",
        &[
            "Left: PNG with an alpha channel (SMask).",
            "Right: JPEG passed through as DCTDecode.",
        ],
    ),
    (
        "Page 3: outline target",
        &[
            "This page exists so the outline has three entries.",
            "The outline points at the top of each page.",
        ],
    ),
];

pub const LINK_URI: &str = "https://example.com";
pub const TITLE: &str = "krilla-fit sample";
pub const PAGE_W: f32 = 595.0;
pub const PAGE_H: f32 = 842.0;

#[cfg(test)]
mod tests {
    use super::jpeg_dimensions;

    #[test]
    fn jpeg_dimensions_skips_fill_bytes_and_standalone_markers() {
        // SOI, fill bytes, TEM, an APP0 with a 2-byte payload, then SOF0 (h = 3, w = 5).
        let data = [
            0xFF, 0xD8, 0xFF, 0xFF, 0xFF, 0x01, 0xFF, 0xE0, 0x00, 0x04, 0xAA, 0xBB, 0xFF, 0xFF, 0xC0, 0x00, 0x0B, 0x08,
            0x00, 0x03, 0x00, 0x05, 0x01, 0x01, 0x11, 0x00,
        ];
        assert_eq!(jpeg_dimensions(&data).ok(), Some((5, 3)));
    }

    #[test]
    fn jpeg_dimensions_rejects_truncated_input() {
        assert!(jpeg_dimensions(&[0xFF, 0xD8, 0xFF]).is_err());
    }
}
