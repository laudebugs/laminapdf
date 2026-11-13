//! Pixels as a PNG file (PNG §11): the signature, `IHDR`, one `IDAT` of the zlib-compressed rows
//! (each with filter 0), and `IEND`. Written here with the core's Flate encoder and PNG chunks
//! (`laminapdf_syntax::png`), so extraction carries no PNG library.

use laminapdf_syntax::filters::flate_encode;
use laminapdf_syntax::png::{SIGNATURE, write_chunk};

use super::color::Output;
use super::samples::Pixels;

/// `pixels` encoded as a PNG.
///
/// # Errors
/// Pixels that don't fill the image's size.
pub(crate) fn encode(pixels: &Pixels) -> Result<Vec<u8>, String> {
    let channels: usize = match (pixels.color, pixels.alpha) {
        (Output::Gray, false) => 1,
        (Output::Gray, true) => 2,
        (Output::Rgb, false) => 3,
        (Output::Rgb, true) => 4,
    };
    let bytes = if pixels.sixteen { 2 } else { 1 };
    let row = pixels.width as usize * channels * bytes;
    if pixels.data.len() != row * pixels.height as usize {
        return Err("the pixels don't fill the image".into());
    }
    let mut raw = Vec::with_capacity((row + 1) * pixels.height as usize);
    for line in pixels.data.chunks(row.max(1)) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let color_type: u8 = match (pixels.color, pixels.alpha) {
        (Output::Gray, false) => 0,
        (Output::Gray, true) => 4,
        (Output::Rgb, false) => 2,
        (Output::Rgb, true) => 6,
    };
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&pixels.width.to_be_bytes());
    header.extend_from_slice(&pixels.height.to_be_bytes());
    header.extend_from_slice(&[if pixels.sixteen { 16 } else { 8 }, color_type, 0, 0, 0]);
    let mut out = SIGNATURE.to_vec();
    write_chunk(&mut out, *b"IHDR", &header);
    write_chunk(&mut out, *b"IDAT", &flate_encode(&raw));
    write_chunk(&mut out, *b"IEND", &[]);
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
        // IHDR's width and height, big-endian, then 8 bits of RGBA.
        assert_eq!(png.get(16..26), Some(&[0, 0, 0, 2, 0, 0, 0, 1, 8, 6][..]));
    }

    #[test]
    fn refuses_pixels_that_dont_fill_the_image() {
        let pixels = Pixels {
            width: 2,
            height: 2,
            color: Output::Gray,
            alpha: false,
            sixteen: false,
            data: vec![0; 3],
        };
        assert!(encode(&pixels).is_err());
    }
}
