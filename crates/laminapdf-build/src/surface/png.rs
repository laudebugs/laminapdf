//! A small PNG decoder ([PNG 3rd edition], W3C; ISO/IEC 15948): chunks with CRCs, `IHDR`,
//! `PLTE`, `tRNS`, the zlib `IDAT` stream (`miniz_oxide`, as `laminapdf-syntax`'s Flate), the
//! five row filters and Adam7 interlacing. It replaces the `png` crate, whose decoder (with
//! `fdeflate`, a second `miniz_oxide` and `crc32fast`) was 48 KB of the 350 KB gzip budget of
//! the WASM base module (ADR 0016 §5). Its output is what the `png` crate gives with
//! `Transformations::EXPAND` (and `STRIP_16` when asked): palettes expanded to RGB, bit
//! depths below 8 scaled to 8, `tRNS` turned into an alpha channel. The tests compare the
//! two on every colour type, bit depth and interlace method.
//!
//! [PNG 3rd edition]: https://www.w3.org/TR/png-3/

use crate::error::{Error, ImageFormat, Result};

use super::image::MAX_DECODED_IMAGE_BYTES;

/// A decoded PNG: interleaved samples, 8 or 16 bits each, gray or RGB, with a trailing alpha
/// channel when `alpha`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Png {
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// One colour channel (gray) or three (RGB).
    pub(crate) gray: bool,
    pub(crate) alpha: bool,
    /// 8 or 16.
    pub(crate) bits: u8,
    pub(crate) pixels: Vec<u8>,
}

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn invalid(reason: &'static str) -> Error {
    Error::InvalidImage {
        format: ImageFormat::Png,
        reason,
    }
}

fn too_large() -> Error {
    Error::ImageTooLarge {
        limit: MAX_DECODED_IMAGE_BYTES,
    }
}

/// The CRC-32 table (ISO 3309, as PNG §5.5 specifies), computed at compile time.
#[expect(
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    reason = "const evaluation: n < 256 indexes the 256-entry table"
)]
const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

/// CRC-32 over `parts` in order.
fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = !0u32;
    for part in parts {
        for &byte in *part {
            let [low, ..] = crc.to_le_bytes();
            let index = usize::from(low ^ byte);
            crc = CRC_TABLE.get(index).copied().unwrap_or(0) ^ (crc >> 8);
        }
    }
    !crc
}

fn be32(b: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(..4)?.try_into().ok()?))
}

/// What `IHDR` says.
#[derive(Debug, Clone, Copy)]
struct Header {
    width: u32,
    height: u32,
    depth: u8,
    color: u8,
    interlaced: bool,
}

impl Header {
    fn parse(data: &[u8]) -> Result<Self> {
        let bad = || invalid("a malformed IHDR chunk");
        let [
            w0,
            w1,
            w2,
            w3,
            h0,
            h1,
            h2,
            h3,
            depth,
            color,
            compression,
            filter,
            interlace,
        ] = <[u8; 13]>::try_from(data).map_err(|_| bad())?;
        let (width, height) = (
            u32::from_be_bytes([w0, w1, w2, w3]),
            u32::from_be_bytes([h0, h1, h2, h3]),
        );
        let depth_ok = match color {
            0 => matches!(depth, 1 | 2 | 4 | 8 | 16),
            3 => matches!(depth, 1 | 2 | 4 | 8),
            2 | 4 | 6 => matches!(depth, 8 | 16),
            _ => false,
        };
        if width == 0 || height == 0 || width > 0x7FFF_FFFF || height > 0x7FFF_FFFF || !depth_ok {
            return Err(bad());
        }
        if compression != 0 || filter != 0 || interlace > 1 {
            return Err(bad());
        }
        Ok(Self {
            width,
            height,
            depth,
            color,
            interlaced: interlace == 1,
        })
    }

    /// Samples per pixel as stored.
    const fn channels(self) -> usize {
        match self.color {
            2 => 3,
            4 => 2,
            6 => 4,
            _ => 1,
        }
    }

    /// Bytes in a stored row of `width` pixels (without the filter byte).
    fn row_bytes(self, width: u32) -> Option<usize> {
        let bits = usize::try_from(width)
            .ok()?
            .checked_mul(self.channels() * usize::from(self.depth))?;
        Some(bits.div_ceil(8))
    }
}

/// The Adam7 passes as (x start, y start, x step, y step), or the whole image.
const ADAM7: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

/// The number of pixels a pass takes from a line of `size`.
const fn pass_size(size: u32, start: u32, step: u32) -> u32 {
    if size > start { (size - start).div_ceil(step) } else { 0 }
}

/// The chunks a decode needs.
struct Chunks<'a> {
    header: Header,
    palette: Option<&'a [u8]>,
    trns: Option<&'a [u8]>,
    idat: Vec<u8>,
}

/// The longest chunk PNG allows (§5.3: lengths are below 2^31).
const MAX_CHUNK: usize = 0x7FFF_FFFF;

/// Reads the chunks, checking each CRC, up to the end of the image data: the first chunk
/// after the `IDAT`s (normally `IEND`) ends the reading, so what follows the image data,
/// even a missing or damaged `IEND`, doesn't matter, as in the `png` crate. A chunk cut short
/// after image data was read ends the reading too; the inflate decides whether the data
/// read is enough.
fn read_chunks(bytes: &[u8]) -> Result<Chunks<'_>> {
    let mut rest = bytes
        .strip_prefix(&SIGNATURE)
        .ok_or_else(|| invalid("no PNG signature"))?;
    let mut header = None;
    let mut palette: Option<&[u8]> = None;
    let mut trns: Option<&[u8]> = None;
    let mut idat = Vec::new();
    while !rest.is_empty() {
        let in_image_data = !idat.is_empty();
        let cut = || invalid("the data ends inside a chunk");
        let parsed = (|| {
            let len = be32(rest).and_then(|l| usize::try_from(l).ok()).ok_or_else(cut)?;
            if len > MAX_CHUNK {
                return Err(invalid("a chunk is longer than PNG allows"));
            }
            let kind = rest.get(4..8).ok_or_else(cut)?;
            let data_end = len.checked_add(8).ok_or_else(cut)?;
            let crc_end = data_end.checked_add(4).ok_or_else(cut)?;
            let data = rest.get(8..data_end).ok_or_else(cut)?;
            let crc = rest.get(data_end..crc_end).and_then(be32).ok_or_else(cut)?;
            Ok((kind, data, crc, crc_end))
        })();
        let (kind, data, crc, end) = match parsed {
            Ok(chunk) => chunk,
            Err(_) if in_image_data => break,
            Err(e) => return Err(e),
        };
        if in_image_data && kind != b"IDAT" {
            break;
        }
        if crc != crc32(&[kind, data]) {
            return Err(invalid("a chunk's CRC does not match"));
        }
        rest = rest.get(end..).unwrap_or_default();
        match kind {
            b"IHDR" if header.is_none() => header = Some(Header::parse(data)?),
            _ if header.is_none() => return Err(invalid("the first chunk is not IHDR")),
            b"PLTE" => {
                if data.is_empty() || data.len() % 3 != 0 || data.len() > 256 * 3 {
                    return Err(invalid("a malformed PLTE chunk"));
                }
                palette = Some(data);
            }
            b"tRNS" => trns = Some(data),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            // An unknown critical chunk (upper-case first letter) can't be skipped (§5.4).
            _ if kind.first().is_some_and(|c| c & 0x20 == 0) => return Err(invalid("an unknown critical chunk")),
            _ => {}
        }
    }
    Ok(Chunks {
        header: header.ok_or_else(|| invalid("no IHDR chunk"))?,
        palette,
        trns,
        idat,
    })
}

/// Inflates zlib `data` into exactly `needed` bytes. Like the `png` crate, a stream that
/// holds more data than the image needs, or ends without (or with a wrong) Adler-32 after
/// the image's bytes, is accepted; fewer bytes than the image needs are an error.
fn inflate(data: &[u8], needed: usize) -> Result<Vec<u8>> {
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

/// Decodes `bytes`; `strip_16` keeps the high byte of 16-bit samples.
pub(crate) fn decode(bytes: &[u8], strip_16: bool) -> Result<Png> {
    let Chunks {
        header: h,
        palette,
        trns,
        idat,
    } = read_chunks(bytes)?;
    let palette = match (h.color, palette) {
        (3, None) => return Err(invalid("an indexed image without a palette")),
        (3, p) => p,
        _ => None,
    };
    let trns = trns.filter(|t| match h.color {
        0 => t.len() == 2,
        2 => t.len() == 6,
        3 => palette.is_some_and(|p| t.len() <= p.len() / 3),
        _ => false,
    });
    let out = Output::new(h, palette, trns, strip_16)?;
    let mut pixels = out.allocate()?;

    // The filtered data of every pass, inflated in one go, at most its exact size.
    let passes: Vec<(u32, u32, u32, u32)> = if h.interlaced {
        ADAM7.to_vec()
    } else {
        vec![(0, 0, 1, 1)]
    };
    let mut needed = 0usize;
    for &(x0, y0, dx, dy) in &passes {
        let (w, rows) = (pass_size(h.width, x0, dx), pass_size(h.height, y0, dy));
        if w > 0 && rows > 0 {
            let row = h.row_bytes(w).ok_or_else(too_large)?;
            needed = (row + 1)
                .checked_mul(usize::try_from(rows).map_err(|_| too_large())?)
                .and_then(|n| n.checked_add(needed))
                .ok_or_else(too_large)?;
        }
    }
    let data = inflate(&idat, needed)?;

    let mut data = data.as_slice();
    for (x0, y0, dx, dy) in passes {
        let (w, rows) = (pass_size(h.width, x0, dx), pass_size(h.height, y0, dy));
        if w == 0 || rows == 0 {
            continue;
        }
        let row_len = h.row_bytes(w).ok_or_else(too_large)?;
        let bpp = (h.channels() * usize::from(h.depth)).div_ceil(8);
        let mut previous = vec![0u8; row_len];
        let mut current = vec![0u8; row_len];
        for r in 0..rows {
            let (filter, rest) = data
                .split_first()
                .ok_or_else(|| invalid("the image data is cut short"))?;
            let raw = rest
                .get(..row_len)
                .ok_or_else(|| invalid("the image data is cut short"))?;
            data = rest.get(row_len..).unwrap_or_default();
            unfilter(*filter, raw, &previous, &mut current, bpp)?;
            let y = y0 + r * dy;
            if !h.interlaced && out.stored_as_output() {
                // Samples already in the output layout: copy the row.
                let at = index(y).saturating_mul(row_len);
                if let Some(dst) = pixels.get_mut(at..at.saturating_add(row_len)) {
                    dst.copy_from_slice(&current);
                }
            } else {
                for i in 0..w {
                    out.put(&mut pixels, &current, i, x0 + i * dx, y);
                }
            }
            std::mem::swap(&mut previous, &mut current);
        }
    }
    Ok(Png {
        width: h.width,
        height: h.height,
        gray: out.gray,
        alpha: out.alpha,
        bits: out.bits,
        pixels,
    })
}

/// Reverses one row's filter (§7.3, §9): `raw` → `out`, given the previous row (zeros for
/// the first) and the bytes per complete pixel (at least 1).
fn unfilter(filter: u8, raw: &[u8], previous: &[u8], out: &mut [u8], bpp: usize) -> Result<()> {
    for i in 0..raw.len() {
        let x = raw.get(i).copied().unwrap_or(0);
        let a = i.checked_sub(bpp).and_then(|j| out.get(j)).copied().unwrap_or(0);
        let b = previous.get(i).copied().unwrap_or(0);
        let c = i.checked_sub(bpp).and_then(|j| previous.get(j)).copied().unwrap_or(0);
        let value = match filter {
            0 => x,
            1 => x.wrapping_add(a),
            2 => x.wrapping_add(b),
            3 => x.wrapping_add(a.midpoint(b)),
            4 => x.wrapping_add(paeth(a, b, c)),
            _ => return Err(invalid("an unknown row filter")),
        };
        if let Some(o) = out.get_mut(i) {
            *o = value;
        }
    }
    Ok(())
}

/// The Paeth predictor (§9.4).
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ia, ib, ic) = (i16::from(a), i16::from(b), i16::from(c));
    let p = ia + ib - ic;
    let (pa, pb, pc) = ((p - ia).abs(), (p - ib).abs(), (p - ic).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// How stored pixels become output pixels.
struct Output<'a> {
    h: Header,
    palette: Option<&'a [u8]>,
    trns: Option<&'a [u8]>,
    gray: bool,
    alpha: bool,
    bits: u8,
    width: usize,
}

impl<'a> Output<'a> {
    fn new(h: Header, palette: Option<&'a [u8]>, trns: Option<&'a [u8]>, strip_16: bool) -> Result<Self> {
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
    fn stored_as_output(&self) -> bool {
        self.h.color != 3 && self.trns.is_none() && self.h.depth >= 8 && self.bits == self.h.depth
    }

    /// Output bytes per pixel.
    fn pixel_bytes(&self) -> usize {
        (usize::from(!self.gray) * 2 + 1 + usize::from(self.alpha)) * usize::from(self.bits / 8)
    }

    /// The zeroed output buffer, refused above [`MAX_DECODED_IMAGE_BYTES`].
    fn allocate(&self) -> Result<Vec<u8>> {
        let size = usize::try_from(self.h.height)
            .ok()
            .and_then(|h| h.checked_mul(self.width))
            .and_then(|n| n.checked_mul(self.pixel_bytes()))
            .filter(|n| *n <= MAX_DECODED_IMAGE_BYTES)
            .ok_or_else(too_large)?;
        Ok(vec![0; size])
    }

    /// Stored sample `index` of `row` (any bit depth), as a 16-bit value.
    fn sample(&self, row: &[u8], index: usize) -> u16 {
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
    fn put(&self, pixels: &mut [u8], row: &[u8], pixel: u32, x: u32, y: u32) {
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

/// A pixel count or coordinate as an index (`u32` always fits in `usize` on our targets).
fn index(v: u32) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::cast_possible_truncation,
        reason = "test code: samples are generated in range"
    )]

    use super::{ADAM7, crc32, decode, paeth, pass_size};

    /// A deterministic pseudo-random byte stream.
    fn noise(seed: u32) -> impl FnMut() -> u16 {
        let mut x = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
        move || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 8) as u16
        }
    }

    fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
        out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        out.extend_from_slice(&kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32(&[&kind, data]).to_be_bytes());
    }

    /// Packs `samples` (one row) at `depth` bits.
    fn pack(samples: &[u16], depth: u8) -> Vec<u8> {
        let mut out = Vec::new();
        match depth {
            16 => samples.iter().for_each(|s| out.extend_from_slice(&s.to_be_bytes())),
            8 => out.extend(samples.iter().map(|s| *s as u8)),
            _ => {
                let mut bits = 0usize;
                for s in samples {
                    if bits.is_multiple_of(8) {
                        out.push(0);
                    }
                    let shift = 8 - usize::from(depth) - bits % 8;
                    *out.last_mut().unwrap() |= (*s as u8) << shift;
                    bits += usize::from(depth);
                }
            }
        }
        out
    }

    /// Filters `row` (bytes) with `filter` against `prev`.
    fn filter_row(filter: u8, row: &[u8], prev: &[u8], bpp: usize) -> Vec<u8> {
        let mut out = vec![filter];
        for i in 0..row.len() {
            let a = if i >= bpp { row[i - bpp] } else { 0 };
            let b = prev[i];
            let c = if i >= bpp { prev[i - bpp] } else { 0 };
            let predicted = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => a.midpoint(b),
                _ => paeth(a, b, c),
            };
            out.push(row[i].wrapping_sub(predicted));
        }
        out
    }

    struct Spec {
        width: u32,
        height: u32,
        color: u8,
        depth: u8,
        interlaced: bool,
        trns: Option<Vec<u8>>,
    }

    /// A PNG with pseudo-random samples, every row filter in turn, and Adam7 when asked.
    fn encode(spec: &Spec, seed: u32) -> Vec<u8> {
        assemble(
            spec,
            &miniz_oxide::deflate::compress_to_vec_zlib(&filtered(spec, seed), 6),
            true,
        )
    }

    /// The filtered image data (before zlib).
    fn filtered(spec: &Spec, seed: u32) -> Vec<u8> {
        let channels = match spec.color {
            2 => 3,
            4 => 2,
            6 => 4,
            _ => 1,
        };
        let max = if spec.color == 3 { 7 } else { (1u32 << spec.depth) - 1 };
        let mut next = noise(seed);
        let pixels: Vec<Vec<u16>> = (0..spec.width * spec.height)
            .map(|_| (0..channels).map(|_| (u32::from(next()) % (max + 1)) as u16).collect())
            .collect();
        let passes: Vec<(u32, u32, u32, u32)> = if spec.interlaced {
            ADAM7.to_vec()
        } else {
            vec![(0, 0, 1, 1)]
        };
        let bpp = (channels * usize::from(spec.depth)).div_ceil(8);
        let mut raw = Vec::new();
        for (x0, y0, dx, dy) in passes {
            let (w, h) = (pass_size(spec.width, x0, dx), pass_size(spec.height, y0, dy));
            if w == 0 || h == 0 {
                continue;
            }
            let mut prev = vec![0u8; (w as usize * channels * usize::from(spec.depth)).div_ceil(8)];
            for r in 0..h {
                let y = y0 + r * dy;
                let samples: Vec<u16> = (0..w)
                    .flat_map(|i| pixels[(y * spec.width + x0 + i * dx) as usize].clone())
                    .collect();
                let row = pack(&samples, spec.depth);
                raw.extend(filter_row((r % 5) as u8, &row, &prev, bpp));
                prev = row;
            }
        }
        raw
    }

    /// The PNG file around zlib data `z`, with or without `IEND`.
    fn assemble(spec: &Spec, z: &[u8], iend: bool) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&spec.width.to_be_bytes());
        ihdr.extend_from_slice(&spec.height.to_be_bytes());
        ihdr.extend_from_slice(&[spec.depth, spec.color, 0, 0, u8::from(spec.interlaced)]);
        chunk(&mut out, *b"IHDR", &ihdr);
        if spec.color == 3 {
            let palette: Vec<u8> = (0..8u8).flat_map(|i| [i * 30, 255 - i * 20, i * 7]).collect();
            chunk(&mut out, *b"PLTE", &palette);
        }
        if let Some(t) = &spec.trns {
            chunk(&mut out, *b"tRNS", t);
        }
        let (a, b) = z.split_at(z.len() / 2);
        chunk(&mut out, *b"IDAT", a);
        chunk(&mut out, *b"IDAT", b);
        if iend {
            chunk(&mut out, *b"IEND", &[]);
        }
        out
    }

    /// The `png` crate's output with `EXPAND` (and `STRIP_16`): (gray, alpha, bits, pixels),
    /// or `None` when it refuses the data.
    fn try_reference(bytes: &[u8], strip: bool) -> Option<(bool, bool, u8, Vec<u8>)> {
        let mut d = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut t = png::Transformations::EXPAND;
        if strip {
            t |= png::Transformations::STRIP_16;
        }
        d.set_transformations(t);
        let mut r = d.read_info().ok()?;
        let mut buf = vec![0; r.output_buffer_size()?];
        let info = r.next_frame(&mut buf).ok()?;
        buf.truncate(info.buffer_size());
        let (gray, alpha) = match info.color_type {
            png::ColorType::Grayscale => (true, false),
            png::ColorType::GrayscaleAlpha => (true, true),
            png::ColorType::Rgb => (false, false),
            _ => (false, true),
        };
        let bits = if info.bit_depth == png::BitDepth::Sixteen {
            16
        } else {
            8
        };
        Some((gray, alpha, bits, buf))
    }

    fn reference(bytes: &[u8], strip: bool) -> (bool, bool, u8, Vec<u8>) {
        try_reference(bytes, strip).unwrap()
    }

    fn ours(bytes: &[u8]) -> Option<(bool, bool, u8, Vec<u8>)> {
        decode(bytes, false).ok().map(|p| (p.gray, p.alpha, p.bits, p.pixels))
    }

    /// Every spec's PNG with its zlib data changed by `change`, decoded by both.
    fn compare_with_changed_zlib(change: impl Fn(Vec<u8>) -> Vec<u8>) {
        for (i, spec) in specs().iter().enumerate() {
            let z = change(miniz_oxide::deflate::compress_to_vec_zlib(&filtered(spec, i as u32), 6));
            let bytes = assemble(spec, &z, true);
            let reference = try_reference(&bytes, false);
            assert!(reference.is_some(), "the png crate refused case {i}");
            assert_eq!(ours(&bytes), reference, "case {i}");
        }
    }

    #[test]
    fn accepts_like_the_png_crate_a_stream_with_more_data_than_the_image() {
        for (i, spec) in specs().iter().enumerate() {
            let mut raw = filtered(spec, i as u32);
            raw.extend_from_slice(&[0; 37]);
            let bytes = assemble(spec, &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6), true);
            assert_eq!(ours(&bytes), try_reference(&bytes, false), "case {i}");
            assert!(ours(&bytes).is_some(), "case {i}");
        }
    }

    #[test]
    fn accepts_like_the_png_crate_a_stream_without_its_adler32() {
        compare_with_changed_zlib(|mut z| {
            z.truncate(z.len() - 4);
            z
        });
    }

    #[test]
    fn decodes_a_file_without_iend_as_the_whole_file() {
        // More lenient than the `png` crate, which refuses it: nothing after the image data
        // matters to the pixels.
        for (i, spec) in specs().iter().enumerate() {
            let z = miniz_oxide::deflate::compress_to_vec_zlib(&filtered(spec, i as u32), 6);
            assert_eq!(
                ours(&assemble(spec, &z, false)),
                ours(&assemble(spec, &z, true)),
                "case {i}"
            );
        }
    }

    #[test]
    fn refuses_like_the_png_crate_truncated_image_data() {
        for (i, spec) in specs().iter().enumerate() {
            let z = miniz_oxide::deflate::compress_to_vec_zlib(&filtered(spec, i as u32), 6);
            let bytes = assemble(spec, &z[..z.len() / 2], true);
            assert_eq!((ours(&bytes), try_reference(&bytes, false)), (None, None), "case {i}");
        }
    }

    #[test]
    fn a_chunk_longer_than_png_allows_is_refused() {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        chunk(&mut out, *b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0]);
        out.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xF0]);
        out.extend_from_slice(b"IDAT");
        assert_eq!(decode(&out, false).unwrap_err().code(), "invalid-image");
    }

    fn specs() -> Vec<Spec> {
        let mut out = Vec::new();
        for (color, depths) in [
            (0u8, &[1u8, 2, 4, 8, 16][..]),
            (2, &[8, 16]),
            (3, &[1, 2, 4, 8]),
            (4, &[8, 16]),
            (6, &[8, 16]),
        ] {
            for &depth in depths {
                for interlaced in [false, true] {
                    let trns = match (color, depth) {
                        (0, d) => Some(vec![0, if d == 16 { 3 } else { 1 }]),
                        (2, _) => Some(vec![0, 1, 0, 2, 0, 3]),
                        (3, _) => Some(vec![0, 128, 255]),
                        _ => None,
                    };
                    for trns in [None, trns] {
                        if trns.is_none()
                            && out
                                .iter()
                                .any(|s: &Spec| s.color == color && s.depth == depth && s.interlaced == interlaced)
                        {
                            continue;
                        }
                        out.push(Spec {
                            width: 13,
                            height: 11,
                            color,
                            depth,
                            interlaced,
                            trns,
                        });
                    }
                }
            }
        }
        out
    }

    #[test]
    fn decodes_like_the_png_crate_for_every_colour_type_depth_and_interlace() {
        for (i, spec) in specs().iter().enumerate() {
            let bytes = encode(spec, i as u32);
            for strip in [false, true] {
                let ours = decode(&bytes, strip).unwrap();
                assert_eq!(
                    (ours.gray, ours.alpha, ours.bits, ours.pixels),
                    reference(&bytes, strip),
                    "colour {} depth {} interlaced {} tRNS {} strip {strip}",
                    spec.color,
                    spec.depth,
                    spec.interlaced,
                    spec.trns.is_some()
                );
            }
        }
    }

    #[test]
    fn a_bad_crc_is_refused() {
        let mut bytes = encode(&specs()[0], 1);
        let last = bytes.len() - 13;
        bytes[last] ^= 1;
        assert_eq!(decode(&bytes, false).unwrap_err().code(), "invalid-image");
    }

    #[test]
    fn truncated_data_is_refused() {
        let bytes = encode(&specs()[3], 1);
        assert_eq!(
            decode(&bytes[..bytes.len() / 2], false).unwrap_err().code(),
            "invalid-image"
        );
    }

    #[test]
    fn an_indexed_image_needs_a_palette() {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let ihdr = [0, 0, 0, 1, 0, 0, 0, 1, 8, 3, 0, 0, 0];
        chunk(&mut out, *b"IHDR", &ihdr);
        chunk(
            &mut out,
            *b"IDAT",
            &miniz_oxide::deflate::compress_to_vec_zlib(&[0, 0], 6),
        );
        chunk(&mut out, *b"IEND", &[]);
        assert_eq!(decode(&out, false).unwrap_err().code(), "invalid-image");
    }

    #[test]
    fn crc32_matches_the_check_value() {
        assert_eq!(crc32(&[b"123456789"]), 0xCBF4_3926);
    }
}
