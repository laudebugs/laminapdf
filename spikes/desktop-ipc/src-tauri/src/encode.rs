//! Lossless tile encoders for the compressed custom-scheme transport.

use crate::tiles::Tile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Webp,
    Qoi,
}

impl Format {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "png" => Some(Self::Png),
            "webp" => Some(Self::Webp),
            "qoi" => Some(Self::Qoi),
            _ => None,
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Webp => "image/webp",
            // No registered type; the webview decodes it in script.
            Self::Qoi => "application/octet-stream",
        }
    }
}

pub fn encode(tile: &Tile, format: Format) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(tile.rgba.len() / 4);
    match format {
        Format::Png => {
            let mut encoder = png::Encoder::new(&mut out, tile.width, tile.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            // The "fast settings" of the todo: fdeflate's fast path with a fixed Up filter.
            encoder.set_compression(png::Compression::Fastest);
            encoder.set_filter(png::Filter::Up);
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer.write_image_data(&tile.rgba).map_err(|e| e.to_string())?;
            writer.finish().map_err(|e| e.to_string())?;
        }
        Format::Webp => {
            image_webp::WebPEncoder::new(&mut out)
                .encode(&tile.rgba, tile.width, tile.height, image_webp::ColorType::Rgba8)
                .map_err(|e| e.to_string())?;
        }
        Format::Qoi => {
            out = qoi::encode_to_vec(&tile.rgba, tile.width, tile.height).map_err(|e| e.to_string())?;
        }
    }
    Ok(out)
}
