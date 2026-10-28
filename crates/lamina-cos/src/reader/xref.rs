//! Cross-reference loading: classic tables (§7.5.4), xref streams (§7.5.8), `/Prev` chains
//! and hybrid `/XRefStm` sections.

use std::collections::{BTreeMap, BTreeSet};

use lamina_syntax::{Dict, Lexer, Object, Parser, Token, filters};

use crate::error::{Error, Result};
use crate::scan::{is_boundary, rfind};

/// Upper bound on `/Prev` hops, so a crafted chain cannot keep us busy.
const MAX_SECTIONS: usize = 4096;

/// Where an object lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XrefEntry {
    Free,
    InFile { offset: usize, gen_nr: u16 },
    InStream { stream: u32, index: u32 },
}

/// The merged result of walking an xref chain.
#[derive(Debug, Default)]
pub(crate) struct XrefTable {
    pub(crate) entries: BTreeMap<u32, XrefEntry>,
    /// The newest trailer (or xref stream dictionary).
    pub(crate) trailer: Dict,
    /// Number of sections reached through `/Prev` (hybrid `/XRefStm` sections do not count).
    pub(crate) sections: usize,
    pub(crate) has_xref_stream: bool,
}

fn invalid(message: &'static str) -> Error {
    Error::InvalidStructure { kind: "xref", message }
}

/// Reads the offset after the last `startxref` keyword.
pub(crate) fn find_startxref(data: &[u8]) -> Result<usize> {
    let pos = rfind(data, b"startxref").ok_or_else(|| invalid("no startxref"))?;
    match Lexer::new(data, pos + b"startxref".len()).next_token()? {
        Token::Integer(off) => usize::try_from(off).map_err(|_| invalid("negative startxref")),
        _ => Err(invalid("startxref is not followed by an offset")),
    }
}

/// Walks the xref chain starting at `start`; newest entries win.
pub(crate) fn load(data: &[u8], start: usize, limit: usize) -> Result<XrefTable> {
    let mut table = XrefTable::default();
    let mut seen = BTreeSet::new();
    let mut next = Some(start);
    while let Some(offset) = next {
        if !seen.insert(offset) || table.sections >= MAX_SECTIONS {
            return Err(invalid("/Prev chain loops"));
        }
        let (mut entries, trailer, is_stream) = parse_section(data, offset, limit)?;
        table.has_xref_stream |= is_stream;
        if let Some(stm) = trailer.get_int("XRefStm").and_then(|o| usize::try_from(o).ok()) {
            // Hybrid file: the classic table's in-use entries win; the stream only fills in
            // numbers the table omits or marks free (compressed objects are typically
            // hidden from pre-1.5 readers that way). A broken XRefStm is ignored.
            if let Ok((stream_entries, _, _)) = parse_section(data, stm, limit) {
                table.has_xref_stream = true;
                for (num, entry) in stream_entries {
                    if matches!(entries.get(&num), None | Some(XrefEntry::Free)) {
                        entries.insert(num, entry);
                    }
                }
            }
        }
        for (num, entry) in entries {
            table.entries.entry(num).or_insert(entry);
        }
        next = trailer.get_int("Prev").and_then(|p| usize::try_from(p).ok());
        if table.sections == 0 {
            table.trailer = trailer;
        }
        table.sections += 1;
    }
    Ok(table)
}

type Section = (BTreeMap<u32, XrefEntry>, Dict, bool);

fn parse_section(data: &[u8], offset: usize, limit: usize) -> Result<Section> {
    let mut lex = Lexer::new(data, offset);
    lex.skip_whitespace();
    let pos = lex.pos();
    if data.get(pos..).is_some_and(|d| d.starts_with(b"xref")) && is_boundary(data, pos + 4) {
        lex.set_pos(pos + 4);
        let (entries, trailer) = parse_table(&mut lex)?;
        return Ok((entries, trailer, false));
    }
    let (_, obj) = Parser::new(data, pos).indirect_object(&mut |_| None)?;
    let Object::Stream(stream) = obj else {
        return Err(invalid("startxref does not point at an xref"));
    };
    if stream.dict.get_name("Type") != Some(b"XRef") {
        return Err(invalid("stream at startxref is not /Type /XRef"));
    }
    let decoded = filters::decode(&stream.dict, &stream.data, limit)?;
    let entries = parse_stream_entries(&stream.dict, &decoded)?;
    Ok((entries, stream.dict, true))
}

fn parse_table(lex: &mut Lexer<'_>) -> Result<(BTreeMap<u32, XrefEntry>, Dict)> {
    let mut entries = BTreeMap::new();
    loop {
        match lex.next_token()? {
            Token::Keyword(b"trailer") => break,
            Token::Integer(start) => {
                let Token::Integer(count) = lex.next_token()? else {
                    return Err(invalid("bad subsection header"));
                };
                let start = u32::try_from(start).map_err(|_| invalid("bad subsection start"))?;
                let count = u32::try_from(count).map_err(|_| invalid("bad subsection count"))?;
                for i in 0..count {
                    // Every iteration consumes three tokens or fails, so a huge count is harmless.
                    let (Token::Integer(off), Token::Integer(g), Token::Keyword(kind)) =
                        (lex.next_token()?, lex.next_token()?, lex.next_token()?)
                    else {
                        return Err(invalid("bad xref entry"));
                    };
                    let num = start.checked_add(i).ok_or_else(|| invalid("object number overflow"))?;
                    let gen_nr = u16::try_from(g).map_err(|_| invalid("bad generation"))?;
                    let entry = match kind {
                        b"n" => XrefEntry::InFile {
                            offset: usize::try_from(off).map_err(|_| invalid("bad offset"))?,
                            gen_nr,
                        },
                        b"f" => XrefEntry::Free,
                        _ => return Err(invalid("entry type must be n or f")),
                    };
                    // Within one table, a later subsection for the same number wins.
                    entries.insert(num, entry);
                }
            }
            _ => return Err(invalid("expected subsection or trailer")),
        }
    }
    match Parser::new(lex.data(), lex.pos()).object()? {
        Object::Dict(d) => Ok((entries, d)),
        _ => Err(invalid("trailer is not a dictionary")),
    }
}

fn read_field(row: &[u8]) -> u64 {
    row.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b))
}

fn parse_stream_entries(dict: &Dict, decoded: &[u8]) -> Result<BTreeMap<u32, XrefEntry>> {
    let widths: Vec<usize> = dict
        .get("W")
        .and_then(Object::as_array)
        .ok_or_else(|| invalid("missing /W"))?
        .iter()
        .map(|w| w.as_int().and_then(|w| usize::try_from(w).ok()).filter(|&w| w <= 8))
        .collect::<Option<_>>()
        .ok_or_else(|| invalid("bad /W"))?;
    let &[w1, w2, w3] = widths.as_slice() else {
        return Err(invalid("/W must have three entries"));
    };
    let row_len = w1 + w2 + w3;
    if row_len == 0 {
        return Err(invalid("zero-width rows"));
    }
    let size = dict.get_int("Size").unwrap_or(0);
    let index: Vec<i64> = match dict.get("Index").and_then(Object::as_array) {
        Some(items) => items
            .iter()
            .map(Object::as_int)
            .collect::<Option<_>>()
            .ok_or_else(|| invalid("bad /Index"))?,
        None => vec![0, size],
    };
    let mut entries = BTreeMap::new();
    let mut rows = decoded.chunks_exact(row_len);
    let (pairs, _) = index.as_chunks::<2>();
    for [start, count] in pairs {
        let start = u32::try_from(*start).map_err(|_| invalid("bad /Index start"))?;
        let count = u32::try_from(*count).map_err(|_| invalid("bad /Index count"))?;
        for i in 0..count {
            // Rows run out before a bogus count does: the loop is bounded by the data.
            let Some(row) = rows.next() else { return Ok(entries) };
            let (f1, rest) = row.split_at(w1);
            let (f2, f3) = rest.split_at(w2);
            let kind = if w1 == 0 { 1 } else { read_field(f1) };
            let (f2, f3) = (read_field(f2), read_field(f3));
            let Some(num) = start.checked_add(i) else {
                return Ok(entries);
            };
            let entry = match kind {
                1 => XrefEntry::InFile {
                    offset: usize::try_from(f2).map_err(|_| invalid("offset overflow"))?,
                    gen_nr: u16::try_from(f3).unwrap_or(u16::MAX),
                },
                2 => XrefEntry::InStream {
                    stream: u32::try_from(f2).map_err(|_| invalid("object stream number overflow"))?,
                    index: u32::try_from(f3).map_err(|_| invalid("index overflow"))?,
                },
                // Type 0 is a free entry. Unknown types are reserved and are treated as
                // references to the null object, which is what a free entry resolves to.
                _ => XrefEntry::Free,
            };
            entries.insert(num, entry);
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hybrid section: table says obj 1 in use at 100 and obj 2 free; the `/XRefStm`
    /// (uncompressed, `/W [1 2 1]`) says both are compressed in object stream 9.
    fn hybrid() -> (Vec<u8>, usize) {
        let mut d = b"%PDF-1.5\n".to_vec();
        let xs = d.len();
        let rows = [0u8, 0, 0, 0, 2, 0, 9, 0, 2, 0, 9, 1];
        d.extend_from_slice(b"7 0 obj << /Type /XRef /W [1 2 1] /Size 3 /Length 12 >> stream\n");
        d.extend_from_slice(&rows);
        d.extend_from_slice(b"\nendstream endobj\n");
        let table = d.len();
        d.extend_from_slice(b"xref\n0 3\n0000000000 65535 f\r\n0000000100 00000 n\r\n0000000000 65535 f\r\n");
        d.extend_from_slice(format!("trailer << /Size 3 /XRefStm {xs} >>\n").as_bytes());
        (d, table)
    }

    #[test]
    fn hybrid_table_in_use_entry_wins_over_xrefstm() {
        let (d, table) = hybrid();
        let entry = load(&d, table, 1024).unwrap().entries.get(&1).copied();
        assert_eq!(entry, Some(XrefEntry::InFile { offset: 100, gen_nr: 0 }));
    }

    #[test]
    fn hybrid_xrefstm_fills_in_entry_the_table_marks_free() {
        let (d, table) = hybrid();
        let entry = load(&d, table, 1024).unwrap().entries.get(&2).copied();
        assert_eq!(entry, Some(XrefEntry::InStream { stream: 9, index: 1 }));
    }

    #[test]
    fn find_startxref_reads_last_offset() {
        assert_eq!(find_startxref(b"startxref\n1\n%%EOF\nstartxref\n42\n%%EOF"), Ok(42));
    }

    #[test]
    fn parse_table_accepts_mixed_eols() {
        let input = b"xref\n0 2\n0000000000 65535 f\r\n0000000017 00000 n \ntrailer\n<< /Size 2 >>";
        let (entries, _, _) = parse_section(input, 0, 1024).unwrap();
        assert_eq!(entries.get(&1), Some(&XrefEntry::InFile { offset: 17, gen_nr: 0 }));
    }

    #[test]
    fn parse_table_rejects_huge_count_without_entries() {
        assert!(parse_section(b"xref\n0 4000000000\ntrailer<<>>", 0, 1024).is_err());
    }

    #[test]
    fn stream_entries_default_type_to_one_when_w1_is_zero() {
        let dict: Dict = [
            ("W", Object::Array(vec![0.into(), 1.into(), 0.into()])),
            ("Size", 1.into()),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            parse_stream_entries(&dict, &[9]).unwrap().get(&0),
            Some(&XrefEntry::InFile { offset: 9, gen_nr: 0 })
        );
    }
}
