//! The chunk layer: the signature, CRC-32 checks, `IHDR` and the chunks a decode reads.

use super::invalid;
use crate::error::Result;

pub(super) const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// The CRC-32 table (ISO 3309, as PNG §5.5 specifies), computed at compile time.
#[expect(
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    reason = "const evaluation: n < 256 indexes the 256-entry table"
)]
pub(super) const CRC_TABLE: [u32; 256] = {
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
pub(super) fn crc32(parts: &[&[u8]]) -> u32 {
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

pub(super) fn be32(b: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(..4)?.try_into().ok()?))
}

/// What `IHDR` says.
#[derive(Debug, Clone, Copy)]
pub(super) struct Header {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) depth: u8,
    pub(super) color: u8,
    pub(super) interlaced: bool,
}

impl Header {
    pub(super) fn parse(data: &[u8]) -> Result<Self> {
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
    pub(super) const fn channels(self) -> usize {
        match self.color {
            2 => 3,
            4 => 2,
            6 => 4,
            _ => 1,
        }
    }

    /// Bytes in a stored row of `width` pixels (without the filter byte).
    pub(super) fn row_bytes(self, width: u32) -> Option<usize> {
        let bits = usize::try_from(width)
            .ok()?
            .checked_mul(self.channels() * usize::from(self.depth))?;
        Some(bits.div_ceil(8))
    }
}

/// The chunks a decode needs.
pub(super) struct Chunks<'a> {
    pub(super) header: Header,
    pub(super) palette: Option<&'a [u8]>,
    pub(super) trns: Option<&'a [u8]>,
    /// `pHYs` and `eXIf`, for how the image is shown.
    pub(super) phys: Option<&'a [u8]>,
    pub(super) exif: Option<&'a [u8]>,
    pub(super) idat: Vec<u8>,
}

/// The longest chunk PNG allows (§5.3: lengths are below 2^31).
pub(super) const MAX_CHUNK: usize = 0x7FFF_FFFF;

/// Reads the chunks, checking each CRC, up to the end of the image data: the first chunk
/// after the `IDAT`s (normally `IEND`) ends the reading, so what follows the image data,
/// even a missing or damaged `IEND`, doesn't matter, as in the `png` crate. A chunk cut short
/// after image data was read ends the reading too; the inflate decides whether the data
/// read is enough.
pub(super) fn read_chunks(bytes: &[u8]) -> Result<Chunks<'_>> {
    let mut rest = bytes
        .strip_prefix(&SIGNATURE)
        .ok_or_else(|| invalid("no PNG signature"))?;
    let mut header = None;
    let mut palette: Option<&[u8]> = None;
    let mut trns: Option<&[u8]> = None;
    let (mut phys, mut exif): (Option<&[u8]>, Option<&[u8]>) = (None, None);
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
            b"pHYs" => phys = Some(data),
            b"eXIf" => exif = Some(data),
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
        phys,
        exif,
        idat,
    })
}
