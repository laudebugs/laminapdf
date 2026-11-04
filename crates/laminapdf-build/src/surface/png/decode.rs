//! The image data: inflating the `IDAT` stream and turning stored pixels into output pixels.

use super::chunks::Header;
use super::{index, invalid, too_large};
use crate::error::Result;
use crate::surface::image::MAX_DECODED_IMAGE_BYTES;

/// Inflates zlib `data` into exactly `needed` bytes. Like the `png` crate, a stream that
/// holds more data than the image needs, or ends without (or with a wrong) Adler-32 after
/// the image's bytes, is accepted; fewer bytes than the image needs are an error.
pub(super) fn inflate(data: &[u8], needed: usize) -> Result<Vec<u8>> {
    use miniz_oxide::inflate::core::{DecompressorOxide, decompress, inflate_flags};
    let mut out = vec![0u8; needed];
    let mut state = DecompressorOxide::new();
    let flags = inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER
        | inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF
        | inflate_flags::TINFL_FLAG_IGNORE_ADLER32;
    let (_, _, written) = decompress(&mut state, data, &mut out, 0, flags);
    if written < needed {
        return Err(invalid(
            "the image data is shorter than the image or not valid zlib data",
        ));
    }
    Ok(out)
}

/// How stored pixels become output pixels.
pub(super) struct Output<'a> {
    pub(super) h: Header,
    pub(super) palette: Option<&'a [u8]>,
    pub(super) trns: Option<&'a [u8]>,
    pub(super) gray: bool,
    pub(super) alpha: bool,
    pub(super) bits: u8,
    pub(super) width: usize,
}

impl<'a> Output<'a> {
    pub(super) fn new(h: Header, palette: Option<&'a [u8]>, trns: Option<&'a [u8]>, strip_16: bool) -> Result<Self> {
        Ok(Self {
            h,
            palette,
            trns,
            gray: matches!(h.color, 0 | 4),
            alpha: matches!(h.color, 4 | 6) || trns.is_some(),
            bits: if h.depth == 16 && !strip_16 { 16 } else { 8 },
            width: usize::try_from(h.width).map_err(|_| too_large())?,
        })
    }

    /// Whether a stored row is already an output row: no palette, no `tRNS`, no bit-depth
    /// change.
    pub(super) fn stored_as_output(&self) -> bool {
        self.h.color != 3 && self.trns.is_none() && self.h.depth >= 8 && self.bits == self.h.depth
    }

    /// Output bytes per pixel.
    pub(super) fn pixel_bytes(&self) -> usize {
        (usize::from(!self.gray) * 2 + 1 + usize::from(self.alpha)) * usize::from(self.bits / 8)
    }

    /// The zeroed output buffer, refused above [`MAX_DECODED_IMAGE_BYTES`].
    pub(super) fn allocate(&self) -> Result<Vec<u8>> {
        let size = usize::try_from(self.h.height)
            .ok()
            .and_then(|h| h.checked_mul(self.width))
            .and_then(|n| n.checked_mul(self.pixel_bytes()))
            .filter(|n| *n <= MAX_DECODED_IMAGE_BYTES)
            .ok_or_else(too_large)?;
        Ok(vec![0; size])
    }

    /// Stored sample `index` of `row` (any bit depth), as a 16-bit value.
    pub(super) fn sample(&self, row: &[u8], index: usize) -> u16 {
        let depth = usize::from(self.h.depth);
        match depth {
            16 => {
                let i = index * 2;
                u16::from_be_bytes([row.get(i).copied().unwrap_or(0), row.get(i + 1).copied().unwrap_or(0)])
            }
            8 => u16::from(row.get(index).copied().unwrap_or(0)),
            _ => {
                let bit = index * depth;
                let byte = row.get(bit / 8).copied().unwrap_or(0);
                let shift = 8 - depth - bit % 8;
                u16::from((byte >> shift) & ((1u8 << depth) - 1))
            }
        }
    }

    /// Writes stored pixel `i` of `row` at (`x`, `y`).
    pub(super) fn put(&self, pixels: &mut [u8], row: &[u8], pixel: u32, x: u32, y: u32) {
        let pixel = index(pixel);
        let channels = self.h.channels();
        // Up to four output samples: colour, then alpha.
        let mut values = [0u16; 4];
        let mut count = 0;
        let mut push = |value: u16| {
            if let Some(slot) = values.get_mut(count) {
                *slot = value;
                count += 1;
            }
        };
        if self.h.color == 3 {
            let entry = usize::from(self.sample(row, pixel)) * 3;
            for k in 0..3 {
                push(u16::from(
                    self.palette.and_then(|p| p.get(entry + k)).copied().unwrap_or(0),
                ));
            }
            if let Some(t) = self.trns {
                push(u16::from(t.get(entry / 3).copied().unwrap_or(0xFF)));
            }
        } else {
            let scale = if self.h.depth < 8 {
                0xFF / ((1u16 << self.h.depth) - 1)
            } else {
                1
            };
            let mut keyed = true;
            for k in 0..channels {
                let value = self.sample(row, pixel * channels + k);
                if let Some(t) = self.trns {
                    let key = u16::from_be_bytes([
                        t.get(2 * k).copied().unwrap_or(0),
                        t.get(2 * k + 1).copied().unwrap_or(0),
                    ]);
                    keyed &= value == key;
                }
                // Below 8 bits (gray only), scaled to 8 by replication, as the `png` crate's
                // EXPAND does.
                push(value * scale);
            }
            if self.trns.is_some() {
                // A colour key (§11.3.2.1): the matching colour is transparent.
                push(if keyed {
                    0
                } else if self.h.depth == 16 {
                    0xFFFF
                } else {
                    0xFF
                });
            }
        }
        let at = (index(y) * self.width + index(x)) * self.pixel_bytes();
        let out = pixels.get_mut(at..).unwrap_or_default();
        let mut out = out.iter_mut();
        for value in values.iter().take(count) {
            let [high, low] = value.to_be_bytes();
            let bytes: &[u8] = if self.bits == 16 {
                &[high, low]
            } else if self.h.depth == 16 {
                &[high]
            } else {
                &[low]
            };
            for b in bytes {
                if let Some(o) = out.next() {
                    *o = *b;
                }
            }
        }
    }
}
