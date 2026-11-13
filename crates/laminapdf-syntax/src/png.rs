//! The PNG container's pieces (PNG §5), shared by the crates that read and write PNGs: the
//! signature, the CRC-32 every chunk carries, and writing a chunk. The row filters are
//! [`filters::unfilter_row`](crate::filters::unfilter_row) and [`filters::paeth`](crate::filters::paeth).

/// The eight bytes every PNG starts with.
///
/// Added in 0.3.0.
pub const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

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

/// CRC-32 over `parts` in order, as a chunk's CRC covers its type and data.
///
/// Added in 0.3.0.
#[must_use]
pub fn crc32(parts: &[&[u8]]) -> u32 {
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

/// Appends chunk `kind` holding `data` to `out`: its length, type, data and CRC.
///
/// Added in 0.3.0.
pub fn write_chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&[&kind, data]).to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_check_value() {
        assert_eq!(crc32(&[b"123456789"]), 0xCBF4_3926);
        // The CRC of IEND with no data, as every PNG ends.
        assert_eq!(crc32(&[b"IEND"]), 0xAE42_6082);
    }
}
