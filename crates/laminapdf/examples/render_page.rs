//! Renders one page of a PDF to an image at the width you ask for. In Rust the image is a PNG
//! file, drawn by hayro, a renderer written in Rust.
//!
//! The renderer is the `laminapdf-render` crate with its `hayro` feature on; the PNG comes from
//! the `png` crate.
//!
//! ```sh
//! cargo run -p laminapdf --example render_page -- <file.pdf> <page index> <width> <out.png>
//! ```

use std::error::Error;
use std::sync::Arc;

use laminapdf_render::{ByteSource, CancelToken, HayroBackend, RenderBackend, RenderOpenOptions, RenderRequest};

const USAGE: &str = "usage: render_page <file.pdf> <page index> <width> <out.png>";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = args.next().ok_or(USAGE)?;
    let index: u32 = args.next().ok_or(USAGE)?.parse()?;
    let width: u16 = args.next().ok_or(USAGE)?.parse()?;
    let out = args.next().ok_or(USAGE)?;
    std::fs::write(out, render_page(std::fs::read(input)?, index, width)?)?;
    Ok(())
}

/// Page `index` of `bytes` as a PNG, `width` pixels wide and as tall as the page needs.
pub(crate) fn render_page(bytes: Vec<u8>, index: u32, width: u16) -> Result<Vec<u8>, Box<dyn Error>> {
    let source: Arc<dyn ByteSource> = Arc::new(bytes);
    let doc = HayroBackend::new().open(source, &RenderOpenOptions::new())?;
    // The page's size in points as it's displayed, its own rotation applied.
    let page = doc.page(index)?;
    // A page's size in pixels rounds down, so ask for half a pixel more to get `width` exactly.
    let request = RenderRequest::page(index, (f32::from(width) + 0.5) / page.width);
    // The plan gives the image's size in pixels, so the buffer is allocated once, at its size.
    let plan = doc.plan(&request)?;
    let mut pixels = vec![0; plan.byte_len()];
    doc.render(&request, &mut pixels, &CancelToken::new())?;
    Ok(png(plan.tile.width, plan.tile.height, &pixels)?)
}

/// Straight RGBA pixels, four bytes each, row by row from the top-left corner, as a PNG.
fn png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, png::EncodingError> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels)?;
    writer.finish()?;
    Ok(out)
}
