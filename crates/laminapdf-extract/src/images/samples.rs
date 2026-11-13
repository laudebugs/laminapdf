//! An image's samples (ISO 32000-2 §8.9.5) turned into pixels: unpacked from 1, 2, 4, 8 or 16
//! bits, mapped by `/Decode`, converted to grey or RGB, with transparency from a soft mask, a
//! stencil mask or a colour-key mask.

use super::color::{Output, Space};

/// Pixels ready for a PNG: grey or RGB, with or without alpha, 8 or 16 bits a channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pixels {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) color: Output,
    pub(crate) alpha: bool,
    pub(crate) sixteen: bool,
    /// The channels, interleaved, big-endian when sixteen bits.
    pub(crate) data: Vec<u8>,
}

/// An image's geometry and sample layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) bits: u8,
}

impl Layout {
    /// The bytes one row of `components` samples takes.
    pub(crate) fn row_bytes(self, components: usize) -> usize {
        (self.width as usize * components * usize::from(self.bits)).div_ceil(8)
    }

    fn pixels(self) -> usize {
        self.width as usize * self.height as usize
    }
}

/// Reads `bits`-bit samples, most significant bit first.
fn sample(row: &[u8], index: usize, bits: u8) -> u16 {
    let byte = |i: usize| row.get(i).copied().unwrap_or(0);
    match bits {
        16 => u16::from_be_bytes([byte(index * 2), byte(index * 2 + 1)]),
        8 => u16::from(byte(index)),
        _ => {
            let bit = index * usize::from(bits);
            let shift = 8 - usize::from(bits) - bit % 8;
            u16::from(byte(bit / 8) >> shift) & ((1 << bits) - 1)
        }
    }
}

/// The raw sample values of each pixel, `components` per pixel, row by row.
pub(crate) fn unpack(data: &[u8], layout: Layout, components: usize) -> Vec<u16> {
    let row_bytes = layout.row_bytes(components);
    let per_row = layout.width as usize * components;
    let mut out = Vec::with_capacity(layout.pixels() * components);
    for y in 0..layout.height as usize {
        let row = data.get(y * row_bytes..).unwrap_or_default();
        out.extend((0..per_row).map(|i| sample(row, i, layout.bits)));
    }
    out
}

/// The `/Decode` ranges of `space`'s components, or its defaults.
pub(crate) fn decode_ranges(space: &Space, decode: Option<&[f64]>, bits: u8) -> Vec<(f64, f64)> {
    (0..space.components())
        .map(|i| match decode {
            Some(d) => match (d.get(2 * i), d.get(2 * i + 1)) {
                (Some(&lo), Some(&hi)) => (lo, hi),
                _ => space.decode_default(i, bits),
            },
            None => space.decode_default(i, bits),
        })
        .collect()
}

/// An image's mask, resampled to the image: each pixel's opacity (0–255), and a soft mask's
/// `/Matte`, the colour the image was blended with (§11.6.5.3), in the image's components.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Mask {
    pub(crate) alpha: Vec<u8>,
    pub(crate) matte: Option<Vec<f64>>,
}

/// Grey pixels, 8 bits, no alpha.
pub(crate) fn gray(layout: Layout, data: Vec<u8>) -> Pixels {
    Pixels {
        width: layout.width,
        height: layout.height,
        color: Output::Gray,
        alpha: false,
        sixteen: false,
        data,
    }
}

/// The pixels of `raw` (from [`unpack`]) in `space`, mapped by `ranges`, with `mask`'s opacity
/// when the image has one. A soft mask's matte is taken out of each component first:
/// c = m + (c − m) / α.
pub(crate) fn pixels(raw: &[u16], layout: Layout, space: &Space, ranges: &[(f64, f64)], mask: Option<&Mask>) -> Pixels {
    let alpha = mask.map(|m| m.alpha.as_slice());
    let matte = mask.and_then(|m| m.matte.as_deref());
    let n = space.components();
    let max = f64::from((1u32 << layout.bits) - 1);
    let color = space.output();
    // Sixteen-bit grey and RGB keep their precision; everything else is 8 bits.
    let sixteen = layout.bits == 16 && matches!(space, Space::Gray | Space::Rgb);
    let channels = match color {
        Output::Gray => 1,
        Output::Rgb => 3,
    } + usize::from(alpha.is_some());
    let mut data = Vec::with_capacity(layout.pixels() * channels * if sixteen { 2 } else { 1 });
    let mut values = Vec::with_capacity(n);
    let mut converted = Vec::with_capacity(3);
    for (p, chunk) in raw.chunks(n).take(layout.pixels()).enumerate() {
        values.clear();
        values.extend(
            chunk
                .iter()
                .zip(ranges)
                .map(|(&v, &(lo, hi))| lo + f64::from(v) * (hi - lo) / max),
        );
        if let (Some(matte), Some(alpha)) = (matte, alpha) {
            let opacity = f64::from(alpha.get(p).copied().unwrap_or(255)) / 255.0;
            if opacity > 0.0 {
                for (v, &m) in values.iter_mut().zip(matte) {
                    *v = m + (*v - m) / opacity;
                }
            }
        }
        converted.clear();
        space.convert(&values, &mut converted);
        for &c in &converted {
            push(&mut data, c, sixteen);
        }
        if let Some(alpha) = alpha {
            push(
                &mut data,
                f64::from(alpha.get(p).copied().unwrap_or(255)) / 255.0,
                sixteen,
            );
        }
    }
    Pixels {
        width: layout.width,
        height: layout.height,
        color,
        alpha: alpha.is_some(),
        sixteen,
        data,
    }
}

fn push(data: &mut Vec<u8>, value: f64, sixteen: bool) {
    let value = value.clamp(0.0, 1.0);
    if sixteen {
        data.extend_from_slice(&scale(value, 65535.0).to_be_bytes());
    } else {
        data.push(u8::try_from(scale(value, 255.0)).unwrap_or(u8::MAX));
    }
}

fn scale(value: f64, top: f64) -> u16 {
    // In 0–top after the clamp, so the cast keeps every value.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0–65535"
    )]
    let scaled = (value * top).round().clamp(0.0, 65535.0) as u16;
    scaled
}

/// A stencil mask's 1-bit samples as opacity: with the default `/Decode [0 1]` a 0 paints and
/// a 1 masks (§8.9.6.2); `inverted` is `/Decode [1 0]`.
pub(crate) fn stencil_alpha(data: &[u8], layout: Layout, inverted: bool) -> Vec<u8> {
    unpack(data, Layout { bits: 1, ..layout }, 1)
        .into_iter()
        .map(|v| if (v == 0) ^ inverted { 255 } else { 0 })
        .collect()
}

/// A soft mask's grey samples as opacity, 0–255.
pub(crate) fn soft_alpha(data: &[u8], layout: Layout, decode: Option<&[f64]>) -> Vec<u8> {
    let (lo, hi) = match decode {
        Some(&[lo, hi, ..]) => (lo, hi),
        _ => (0.0, 1.0),
    };
    let max = f64::from((1u32 << layout.bits) - 1);
    unpack(data, layout, 1)
        .into_iter()
        .map(|v| {
            let opacity = (lo + f64::from(v) * (hi - lo) / max).clamp(0.0, 1.0);
            u8::try_from(scale(opacity, 255.0)).unwrap_or(u8::MAX)
        })
        .collect()
}

/// A colour-key mask's opacity (§8.9.6.4): a pixel whose every raw component is in its
/// `[min max]` range is transparent.
pub(crate) fn color_key_alpha(raw: &[u16], components: usize, ranges: &[(u16, u16)]) -> Vec<u8> {
    raw.chunks(components)
        .map(|pixel| {
            let masked =
                pixel.len() == ranges.len() && pixel.iter().zip(ranges).all(|(&v, &(lo, hi))| (lo..=hi).contains(&v));
            if masked { 0 } else { 255 }
        })
        .collect()
}

/// `alpha` for a `from`-sized mask, resampled to `to` by the nearest pixel.
pub(crate) fn resample(alpha: &[u8], from: Layout, to: Layout) -> Vec<u8> {
    if (from.width, from.height) == (to.width, to.height) {
        return alpha.to_vec();
    }
    let (fw, fh) = (from.width.max(1) as usize, from.height.max(1) as usize);
    let (tw, th) = (to.width as usize, to.height as usize);
    let mut out = Vec::with_capacity(tw * th);
    for y in 0..th {
        let sy = y * fh / th.max(1);
        for x in 0..tw {
            let sx = x * fw / tw.max(1);
            out.push(alpha.get(sy * fw + sx).copied().unwrap_or(255));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn layout(width: u32, height: u32, bits: u8) -> Layout {
        Layout { width, height, bits }
    }

    #[test]
    fn unpacks_samples_of_every_width() {
        assert_eq!(unpack(&[0b1010_0000], layout(3, 1, 1), 1), vec![1, 0, 1]);
        assert_eq!(unpack(&[0b1101_0010], layout(2, 1, 4), 1), vec![13, 2]);
        assert_eq!(unpack(&[0x12, 0x34], layout(1, 1, 16), 1), vec![0x1234]);
        // Each row starts on a byte.
        assert_eq!(
            unpack(&[0b1000_0000, 0b0100_0000], layout(2, 2, 1), 1),
            vec![1, 0, 0, 1]
        );
    }

    #[test]
    fn an_inverted_decode_flips_grey() {
        let ranges = decode_ranges(&Space::Gray, Some(&[1.0, 0.0]), 8);
        let out = pixels(&[0, 255], layout(2, 1, 8), &Space::Gray, &ranges, None);
        assert_eq!(out.data, vec![255, 0]);
    }

    #[test]
    fn sixteen_bit_rgb_keeps_sixteen_bits() {
        let ranges = decode_ranges(&Space::Rgb, None, 16);
        let out = pixels(&[65535, 0, 32768], layout(1, 1, 16), &Space::Rgb, &ranges, None);
        assert!(out.sixteen);
        assert_eq!(out.data, vec![0xff, 0xff, 0, 0, 0x80, 0]);
    }

    #[test]
    fn a_stencil_paints_its_zeros() {
        assert_eq!(stencil_alpha(&[0b0100_0000], layout(2, 1, 1), false), vec![255, 0]);
        assert_eq!(stencil_alpha(&[0b0100_0000], layout(2, 1, 1), true), vec![0, 255]);
    }

    #[test]
    fn a_color_key_hides_the_pixels_in_its_ranges() {
        assert_eq!(color_key_alpha(&[1, 5, 9], 1, &[(4, 6)]), vec![255, 0, 255]);
    }

    #[test]
    fn a_smaller_mask_is_resampled_to_the_image() {
        let alpha = resample(&[0, 255], layout(2, 1, 8), layout(4, 1, 8));
        assert_eq!(alpha, vec![0, 0, 255, 255]);
    }
}
