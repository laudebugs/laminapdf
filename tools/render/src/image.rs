//! RGB bitmaps: PPM in (PDFium's renders from `pdfium.py`), PNG in and out (the goldens).

use std::fs;
use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// An opaque 8-bit RGB bitmap, rows top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgb {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Rgb {
    /// A `width`×`height` bitmap of `data`, checked to be exactly `width * height * 3` bytes.
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        let expected = usize::try_from(u64::from(width) * u64::from(height) * 3)?;
        if data.len() != expected {
            bail!("{width}x{height} RGB needs {expected} bytes, got {}", data.len());
        }
        Ok(Self { width, height, data })
    }

    /// The pixel at `(x, y)`, or black outside the bitmap.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let at = (u64::from(y) * u64::from(self.width) + u64::from(x)) * 3;
        usize::try_from(at)
            .ok()
            .and_then(|at| self.data.get(at..at + 3))
            .and_then(|p| <[u8; 3]>::try_from(p).ok())
            .unwrap_or([0; 3])
    }

    /// The top-left `width`×`height` part (both no larger than this bitmap's).
    pub fn crop(&self, width: u32, height: u32) -> Self {
        let mut data = Vec::new();
        for y in 0..height.min(self.height) {
            for x in 0..width.min(self.width) {
                data.extend(self.pixel(x, y));
            }
        }
        Self {
            width: width.min(self.width),
            height: height.min(self.height),
            data,
        }
    }
}

/// Reads a binary PPM (`P6`, maxval 255) as `pdfium.py` writes it.
pub fn read_ppm(path: &Path) -> Result<Rgb> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut fields = Vec::new();
    let mut at = 0;
    // Four whitespace-separated header fields, then one whitespace byte, then the pixels.
    while fields.len() < 4 {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        let start = at;
        while bytes.get(at).is_some_and(|b| !b.is_ascii_whitespace()) {
            at += 1;
        }
        let field = bytes.get(start..at).context("truncated PPM header")?;
        fields.push(String::from_utf8_lossy(field).into_owned());
    }
    let [magic, width, height, max] = fields.as_slice() else {
        bail!("bad PPM header in {}", path.display());
    };
    if magic != "P6" || max != "255" {
        bail!("{}: not an 8-bit binary PPM", path.display());
    }
    let data = bytes.get(at + 1..).context("truncated PPM")?.to_vec();
    Rgb::new(width.parse()?, height.parse()?, data).with_context(|| path.display().to_string())
}

/// Reads an 8-bit RGB PNG (a golden).
pub fn read_png(path: &Path) -> Result<Rgb> {
    let file = fs::File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file)).read_info()?;
    let mut data = vec![0; reader.output_buffer_size().context("PNG too large")?];
    let info = reader.next_frame(&mut data)?;
    if info.color_type != png::ColorType::Rgb || info.bit_depth != png::BitDepth::Eight {
        bail!("{}: not an 8-bit RGB PNG", path.display());
    }
    data.truncate(info.buffer_size());
    Rgb::new(info.width, info.height, data)
}

/// Writes `img` as an 8-bit RGB PNG, compressed hard (goldens are committed).
pub fn write_png(path: &Path, img: &Rgb) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let file = BufWriter::new(fs::File::create(path).with_context(|| format!("writing {}", path.display()))?);
    let mut encoder = png::Encoder::new(file, img.width, img.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    encoder.write_header()?.write_image_data(&img.data)?;
    Ok(())
}
