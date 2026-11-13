//! Pixels as a PNG file.

use super::color::Output;
use super::samples::Pixels;

/// `pixels` encoded as a PNG.
///
/// # Errors
/// The encoder's, as text: pixels that don't fill the image's size.
pub(crate) fn encode(pixels: &Pixels) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, pixels.width, pixels.height);
        encoder.set_color(match (pixels.color, pixels.alpha) {
            (Output::Gray, false) => png::ColorType::Grayscale,
            (Output::Gray, true) => png::ColorType::GrayscaleAlpha,
            (Output::Rgb, false) => png::ColorType::Rgb,
            (Output::Rgb, true) => png::ColorType::Rgba,
        });
        encoder.set_depth(if pixels.sixteen {
            png::BitDepth::Sixteen
        } else {
            png::BitDepth::Eight
        });
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&pixels.data).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_png_signature_and_its_size() {
        let pixels = Pixels {
            width: 2,
            height: 1,
            color: Output::Rgb,
            alpha: true,
            sixteen: false,
            data: vec![255, 0, 0, 255, 0, 0, 255, 128],
        };
        let png = encode(&pixels).unwrap();
        assert_eq!(png.get(..8), Some(&b"\x89PNG\r\n\x1a\n"[..]));
        // IHDR's width and height, big-endian.
        assert_eq!(png.get(16..24), Some(&[0, 0, 0, 2, 0, 0, 0, 1][..]));
    }
}
