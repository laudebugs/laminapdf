//! Pixels as a PNG file (PNG §11): the signature, `IHDR`, one `IDAT` of the zlib-compressed rows
//! (each with filter 0), and `IEND`. Written here with the Flate encoder the core already has,
//! so extraction carries no PNG library.

use laminapdf_syntax::filters::flate_encode;

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
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, *b"IHDR", &header);
    chunk(&mut out, *b"IDAT", &flate_encode(&raw));
    chunk(&mut out, *b"IEND", &[]);
    Ok(out)
}

/// Appends chunk `kind` holding `data`: its length, type, data and CRC.
fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    let start = out.len();
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    let crc = crc32(out.get(start..).unwrap_or_default());
    out.extend_from_slice(&crc.to_be_bytes());
}

/// CRC-32 (ISO 3309, as PNG §D computes it).
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
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
    fn crc32_is_the_png_one() {
        // The CRC of IEND with no data, as every PNG ends.
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
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
