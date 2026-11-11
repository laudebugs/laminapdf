//! Just enough of the OpenType file format (sfnt) to list a font file's faces and copy one out:
//! the collection header, a face's table directory, `name` IDs 1 and 2, and `OS/2`'s weight,
//! style and code pages (<https://learn.microsoft.com/en-us/typography/opentype/spec/otff>).
//! Reads go through a closure, so listing reads a few kilobytes per face, never whole files.

use std::io;

use laminapdf_desktop_protocol::Charsets;

/// One table record of a face: its tag, where it starts in the file, and its length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Table {
    pub(super) tag: [u8; 4],
    pub(super) offset: u32,
    pub(super) len: u32,
}

/// What a face says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Described {
    pub(super) family: String,
    pub(super) style: String,
    pub(super) weight: u16,
    pub(super) italic: bool,
    pub(super) charsets: Charsets,
}

/// Reads `len` bytes at `offset` of the file.
pub(super) type ReadAt<'a> = dyn FnMut(u64, usize) -> io::Result<Vec<u8>> + 'a;

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_owned())
}

fn array_at<const N: usize>(bytes: &[u8], at: usize) -> io::Result<[u8; N]> {
    bytes
        .get(at..at + N)
        .and_then(|slice| <[u8; N]>::try_from(slice).ok())
        .ok_or_else(|| invalid("truncated table"))
}

fn u16_at(bytes: &[u8], at: usize) -> io::Result<u16> {
    array_at(bytes, at).map(u16::from_be_bytes)
}

fn u32_at(bytes: &[u8], at: usize) -> io::Result<u32> {
    array_at(bytes, at).map(u32::from_be_bytes)
}

/// Where each face's table directory starts: one face at 0, or a collection's faces.
pub(super) fn faces(read: &mut ReadAt<'_>) -> io::Result<Vec<u32>> {
    let header = read(0, 12)?;
    match header.get(..4) {
        Some(b"ttcf") => {
            let count = u32_at(&header, 8)?.min(256);
            let offsets = read(12, count as usize * 4)?;
            (0..count as usize).map(|i| u32_at(&offsets, i * 4)).collect()
        }
        Some([0, 1, 0, 0] | b"OTTO" | b"true") => Ok(vec![0]),
        _ => Err(invalid("not an OpenType font")),
    }
}

/// The table directory of the face at `offset`.
pub(super) fn tables(read: &mut ReadAt<'_>, offset: u32) -> io::Result<Vec<Table>> {
    let header = read(u64::from(offset), 12)?;
    let count = usize::from(u16_at(&header, 4)?).min(512);
    let records = read(u64::from(offset) + 12, count * 16)?;
    (0..count)
        .map(|i| {
            let at = i * 16;
            Ok(Table {
                tag: array_at(&records, at)?,
                offset: u32_at(&records, at + 8)?,
                len: u32_at(&records, at + 12)?,
            })
        })
        .collect()
}

/// The face's names, weight, style and code pages.
pub(super) fn describe(read: &mut ReadAt<'_>, tables: &[Table]) -> io::Result<Described> {
    let find = |tag: &[u8; 4]| tables.iter().find(|t| &t.tag == tag).copied();
    let name = find(b"name").ok_or_else(|| invalid("no name table"))?;
    // Names sit near the start of `name`; 64 KiB holds every one a font lists in practice.
    let names = read(u64::from(name.offset), (name.len as usize).min(64 << 10))?;
    let family = name_string(&names, 1).ok_or_else(|| invalid("no family name"))?;
    let style = name_string(&names, 2).unwrap_or_else(|| "Regular".into());
    let mut described = Described {
        family,
        italic: style.contains("Italic") || style.contains("Oblique"),
        weight: if style.contains("Bold") { 700 } else { 400 },
        style,
        charsets: Charsets::ANSI,
    };
    if let Some(os2) = find(b"OS/2") {
        let os2 = read(u64::from(os2.offset), (os2.len as usize).min(100))?;
        described.weight = u16_at(&os2, 4).unwrap_or(described.weight);
        let selection = u16_at(&os2, 62).unwrap_or(0);
        described.italic |= selection & 1 != 0 || selection & (1 << 9) != 0;
        if let Ok(pages) = u32_at(&os2, 78) {
            described.charsets = charsets(pages);
        }
    }
    Ok(described)
}

/// PDFium's reading of `ulCodePageRange1` (its folder font info's): every face counts as Latin.
fn charsets(pages: u32) -> Charsets {
    let bits = [
        (17, Charsets::SHIFT_JIS),
        (18, Charsets::GB2312),
        (20, Charsets::BIG5),
        (19, Charsets::HANGUL),
        (21, Charsets::HANGUL),
        (31, Charsets::SYMBOL),
    ];
    bits.iter()
        .filter(|(bit, _)| pages & (1 << bit) != 0)
        .fold(Charsets::ANSI, |all, (_, set)| all.with(*set))
}

/// `name` ID `id`: the Windows English (US) record if there is one, else Unicode, else Mac Roman.
fn name_string(names: &[u8], id: u16) -> Option<String> {
    let count = usize::from(u16_at(names, 2).ok()?);
    let strings = usize::from(u16_at(names, 4).ok()?);
    let mut best: Option<(u8, String)> = None;
    for i in 0..count {
        let at = 6 + i * 12;
        let (platform, encoding, language, name_id) = (
            u16_at(names, at).ok()?,
            u16_at(names, at + 2).ok()?,
            u16_at(names, at + 4).ok()?,
            u16_at(names, at + 6).ok()?,
        );
        if name_id != id {
            continue;
        }
        let len = usize::from(u16_at(names, at + 8).ok()?);
        let start = strings + usize::from(u16_at(names, at + 10).ok()?);
        let Some(raw) = names.get(start..start + len) else {
            continue;
        };
        let (rank, text) = match (platform, encoding, language) {
            (3, 1 | 10, 0x409) => (3, utf16(raw)),
            (3 | 0, ..) => (2, utf16(raw)),
            (1, 0, 0) => (1, raw.iter().map(|&b| char::from(b)).collect()),
            _ => continue,
        };
        if best.as_ref().is_none_or(|(have, _)| rank > *have) && !text.is_empty() {
            best = Some((rank, text));
        }
    }
    best.map(|(_, text)| text)
}

fn utf16(raw: &[u8]) -> String {
    let units: Vec<u16> = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_be_bytes(*pair))
        .collect();
    String::from_utf16_lossy(&units)
}

/// The face at `offset` copied out as a standalone font file: its own table directory and its
/// tables, each 4-byte aligned. PDFium then loads exactly that face, even from a collection.
pub(super) fn extract(read: &mut ReadAt<'_>, offset: u32, tables: &[Table]) -> io::Result<Vec<u8>> {
    let version = read(u64::from(offset), 4)?;
    let count = u16::try_from(tables.len()).map_err(|_| invalid("too many tables"))?;
    let mut power = 1u16;
    let mut log2 = 0u16;
    while power.saturating_mul(2) <= count {
        power *= 2;
        log2 += 1;
    }
    let search = power.saturating_mul(16);
    let mut out = Vec::new();
    out.extend_from_slice(&version);
    for value in [count, search, log2, count.saturating_mul(16).saturating_sub(search)] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    let directory_end = 12 + tables.len() * 16;
    let mut data = Vec::new();
    for table in tables {
        let start = u32::try_from(directory_end + data.len()).map_err(|_| invalid("face too large"))?;
        out.extend_from_slice(&table.tag);
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&start.to_be_bytes());
        out.extend_from_slice(&table.len.to_be_bytes());
        data.extend_from_slice(&read(u64::from(table.offset), table.len as usize)?);
        data.resize(data.len().next_multiple_of(4), 0);
    }
    out.extend_from_slice(&data);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_page_bits_should_map_to_pdfiums_charsets() {
        let set = charsets((1 << 18) | (1 << 21));
        assert_eq!(set, Charsets::ANSI.with(Charsets::GB2312).with(Charsets::HANGUL));
    }
}
