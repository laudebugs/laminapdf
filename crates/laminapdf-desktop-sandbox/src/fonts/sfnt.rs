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

/// Fills the buffer with the file's bytes at the offset.
pub(super) type ReadAt<'a> = dyn FnMut(u64, &mut [u8]) -> io::Result<()> + 'a;

/// The longest family or style name kept, in characters: real names are a few dozen, and the list
/// goes to every engine in one frame.
pub(super) const MAX_NAME: usize = 256;

/// `len` bytes at `offset`, in a new buffer (the small reads: headers, directories, names).
fn read_vec(read: &mut ReadAt<'_>, offset: u64, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    read(offset, &mut buf)?;
    Ok(buf)
}

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
    let header = read_vec(read, 0, 12)?;
    match header.get(..4) {
        Some(b"ttcf") => {
            let count = u32_at(&header, 8)?.min(256);
            let offsets = read_vec(read, 12, count as usize * 4)?;
            (0..count as usize).map(|i| u32_at(&offsets, i * 4)).collect()
        }
        Some([0, 1, 0, 0] | b"OTTO" | b"true") => Ok(vec![0]),
        _ => Err(invalid("not an OpenType font")),
    }
}

/// The table directory of the face at `offset`.
pub(super) fn tables(read: &mut ReadAt<'_>, offset: u32) -> io::Result<Vec<Table>> {
    let header = read_vec(read, u64::from(offset), 12)?;
    let count = usize::from(u16_at(&header, 4)?).min(512);
    let records = read_vec(read, u64::from(offset) + 12, count * 16)?;
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
    let names = read_vec(read, u64::from(name.offset), (name.len as usize).min(64 << 10))?;
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
        let os2 = read_vec(read, u64::from(os2.offset), (os2.len as usize).min(100))?;
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
    best.map(|(_, text)| text.chars().take(MAX_NAME).collect())
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
/// tables, each 4-byte aligned. PDFium then loads exactly that face, even from a collection. The
/// tables are read straight into the output, sized once, so a large face is held once.
pub(super) fn extract(read: &mut ReadAt<'_>, offset: u32, tables: &[Table]) -> io::Result<Vec<u8>> {
    let version = read_vec(read, u64::from(offset), 4)?;
    let count = u16::try_from(tables.len()).map_err(|_| invalid("too many tables"))?;
    let mut power = 1u16;
    let mut log2 = 0u16;
    while power.saturating_mul(2) <= count {
        power *= 2;
        log2 += 1;
    }
    let search = power.saturating_mul(16);
    let directory_end = 12 + tables.len() * 16;
    let data_len: usize = tables.iter().map(|t| (t.len as usize).next_multiple_of(4)).sum();
    let mut out = Vec::with_capacity(directory_end + data_len);
    out.extend_from_slice(&version);
    for value in [count, search, log2, count.saturating_mul(16).saturating_sub(search)] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    let mut start = directory_end;
    for table in tables {
        out.extend_from_slice(&table.tag);
        out.extend_from_slice(&[0; 4]);
        let at = u32::try_from(start).map_err(|_| invalid("face too large"))?;
        out.extend_from_slice(&at.to_be_bytes());
        out.extend_from_slice(&table.len.to_be_bytes());
        start += (table.len as usize).next_multiple_of(4);
    }
    for table in tables {
        let at = out.len();
        out.resize(at + (table.len as usize).next_multiple_of(4), 0);
        let slot = out
            .get_mut(at..at + table.len as usize)
            .ok_or_else(|| invalid("face too large"))?;
        read(u64::from(table.offset), slot)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A collection of two faces from the fixtures (Noto Sans regular and bold, OFL).
    fn collection() -> Vec<u8> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/fonts/NotoSans-RegularBold.ttc"
        );
        std::fs::read(path).unwrap_or_default()
    }

    /// Lists and copies out every face of `bytes`, as the main process does, ignoring errors: a
    /// malformed file may fail, never panic.
    fn read_all(bytes: &[u8]) -> usize {
        let mut read = |offset: u64, buf: &mut [u8]| {
            let start = usize::try_from(offset).map_err(|_| invalid("offset"))?;
            let end = start.checked_add(buf.len()).ok_or_else(|| invalid("length"))?;
            let src = bytes.get(start..end).ok_or_else(|| invalid("past the end"))?;
            buf.copy_from_slice(src);
            Ok(())
        };
        let mut served = 0;
        for offset in faces(&mut read).unwrap_or_default() {
            let Ok(found) = tables(&mut read, offset) else { continue };
            // The main process skips faces past its size cap before copying them.
            if found.iter().map(|t| u64::from(t.len)).sum::<u64>() > 1 << 20 {
                continue;
            }
            if describe(&mut read, &found).is_ok() && extract(&mut read, offset, &found).is_ok() {
                served += 1;
            }
        }
        served
    }

    #[test]
    fn both_faces_of_a_collection_should_be_listed_and_copied_out() {
        assert_eq!(read_all(&collection()), 2);
    }

    #[test]
    fn a_truncated_file_should_fail_without_panicking() {
        let bytes = collection();
        for len in (0..bytes.len()).step_by(97) {
            let _ = read_all(bytes.get(..len).unwrap_or_default());
        }
    }

    #[test]
    fn flipped_bytes_in_the_headers_should_fail_without_panicking() {
        let bytes = collection();
        // The collection header, both table directories and the start of the tables.
        for at in 0..bytes.len().min(2048) {
            for flip in [0x01, 0x80, 0xFF] {
                let mut broken = bytes.clone();
                if let Some(byte) = broken.get_mut(at) {
                    *byte ^= flip;
                }
                let _ = read_all(&broken);
            }
        }
    }

    #[test]
    fn a_long_name_should_be_cut_to_the_cap() {
        let name: String = "x".repeat(MAX_NAME * 4);
        let utf16: Vec<u8> = name.encode_utf16().flat_map(u16::to_be_bytes).collect();
        // A `name` table with one Windows English record for ID 1.
        let mut table = vec![0, 0, 0, 1, 0, 18];
        let len = u16::try_from(utf16.len()).unwrap_or(u16::MAX);
        for value in [3u16, 1, 0x409, 1, len, 0] {
            table.extend_from_slice(&value.to_be_bytes());
        }
        table.extend_from_slice(&utf16);
        assert_eq!(name_string(&table, 1).map(|n| n.chars().count()), Some(MAX_NAME));
    }

    #[test]
    fn code_page_bits_should_map_to_pdfiums_charsets() {
        let set = charsets((1 << 18) | (1 << 21));
        assert_eq!(set, Charsets::ANSI.with(Charsets::GB2312).with(Charsets::HANGUL));
    }
}
