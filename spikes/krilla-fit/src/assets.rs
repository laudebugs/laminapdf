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

/// Width and height from the first SOFn marker.
pub fn jpeg_dimensions(data: &[u8]) -> Result<(u32, u32)> {
    let mut i = 2;
    while i + 9 < data.len() {
        if data[i] != 0xFF {
            bail!("bad JPEG marker at {i}");
        }
        let marker = data[i + 1];
        let len = usize::from(u16::from_be_bytes([data[i + 2], data[i + 3]]));
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            let h = u16::from_be_bytes([data[i + 5], data[i + 6]]);
            let w = u16::from_be_bytes([data[i + 7], data[i + 8]]);
            return Ok((u32::from(w), u32::from(h)));
        }
        i += 2 + len;
    }
    bail!("no SOF marker in JPEG")
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
