//! Cross-reference loading: classic tables (§7.5.4), xref streams (§7.5.8), `/Prev` chains
//! and hybrid `/XRefStm` sections.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_syntax::{DecodeBudget, Dict, Limits, Object, Parser, Token, filters};

use super::window::{Reach, parse_syntax_at, read_indirect, rfind_input};
use crate::error::{Error, Result};
use crate::source::Source;

/// Trailer keys that only describe an xref *section* (classic table or xref stream), never the
/// document: they are not carried over into a rebuilt or appended trailer (`/Size` is
/// recomputed by whoever writes the new one).
///
/// Added in 0.2.0.
pub const XREF_STREAM_KEYS: &[&str] = &[
    "Size",
    "Prev",
    "Type",
    "W",
    "Index",
    "Filter",
    "DecodeParms",
    "Length",
    "XRefStm",
    // Stream-dictionary keys an xref stream may carry (§7.3.8.2): external data, its
    // filters, and the decoded length.
    "F",
    "FFilter",
    "FDecodeParms",
    "DL",
];

/// Where an object lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XrefEntry {
    Free,
    InFile { offset: usize, gen_nr: u16 },
    InStream { stream: u32, index: u32 },
}

impl XrefEntry {
    /// The generation number of the object this entry describes (objects in object streams
    /// always have generation 0, §7.5.7).
    pub(crate) const fn gen_nr(self) -> u16 {
        match self {
            Self::InFile { gen_nr, .. } => gen_nr,
            Self::Free | Self::InStream { .. } => 0,
        }
    }

    /// Whether a reference with generation `gen_nr` refers to this entry's object. A
    /// mismatch means the reference is stale and resolves to `null` (§7.3.10).
    pub(crate) const fn matches_gen(self, gen_nr: u16) -> bool {
        match self {
            Self::Free => false,
            Self::InFile { .. } | Self::InStream { .. } => self.gen_nr() == gen_nr,
        }
    }
}

/// The merged result of walking an xref chain.
#[derive(Debug, Default)]
pub(crate) struct XrefTable {
    pub(crate) entries: BTreeMap<u32, XrefEntry>,
    /// The newest trailer (or xref stream dictionary).
    pub(crate) trailer: Dict,
    /// Number of sections reached through `/Prev` (hybrid `/XRefStm` sections do not count).
    pub(crate) sections: usize,
    /// Their offsets, newest first.
    pub(crate) section_offsets: Vec<usize>,
    /// What each of those sections lists, in the same order (a hybrid section's `/XRefStm`
    /// entries count with it).
    pub(crate) section_objects: Vec<SectionObjects>,
    pub(crate) has_xref_stream: bool,
    /// Whether the newest section (the one `startxref` points at) is an xref stream.
    pub(crate) newest_is_stream: bool,
    /// Offsets of hybrid `/XRefStm` sections that could not be read (`xrefstm-ignored`).
    pub(crate) ignored_xref_streams: Vec<u64>,
    /// Entries dropped for exceeding `Limits::max_object_number` (`object-number-out-of-range`).
    pub(crate) dropped_objects: BTreeSet<u32>,
    /// Whether a trailer or xref-stream dictionary in the chain had something other than a
    /// name where a key belongs, which was skipped (`dictionary-key-skipped`).
    pub(crate) skipped_trailer_keys: bool,
    /// The head of the free list (§7.5.4): object 0's next-free number in the newest section
    /// that lists object 0.
    pub(crate) free_head: Option<u32>,
}

/// The object numbers one cross-reference section lists in use and free. Object 0, the head
/// of the free list (§7.5.4), is never counted as freed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SectionObjects {
    pub(crate) in_use: Vec<u32>,
    pub(crate) freed: Vec<u32>,
}

impl SectionObjects {
    fn of(entries: &BTreeMap<u32, XrefEntry>, limits: &Limits) -> Self {
        let mut objects = Self::default();
        for (&num, entry) in entries {
            if num > limits.max_object_number() {
                continue;
            }
            match entry {
                XrefEntry::Free if num != 0 => objects.freed.push(num),
                XrefEntry::Free => {}
                XrefEntry::InFile { .. } | XrefEntry::InStream { .. } => objects.in_use.push(num),
            }
        }
        objects
    }
}

fn invalid(message: &'static str) -> Error {
    Error::InvalidStructure { kind: "xref", message }
}

/// Reads the offset after the last `startxref` keyword.
///
/// # Errors
/// [`Error::InvalidStructure`] when there is no usable `startxref`; the source's errors.
pub(crate) fn find_startxref(source: &Source, limits: &Limits) -> Result<usize> {
    let pos = rfind_input(source, b"startxref")?.ok_or_else(|| invalid("no startxref"))?;
    let token = parse_syntax_at(source, pos + b"startxref".len(), Reach::Object, limits, |p, _| {
        Ok(match p.lexer().next_token()? {
            Token::Integer(off) => Some(off),
            _ => None,
        })
    })??;
    match token {
        Some(off) => usize::try_from(off).map_err(|_| invalid("negative startxref")),
        None => Err(invalid("startxref is not followed by an offset")),
    }
}

/// Walks the xref chain starting at `start`; newest entries win.
///
/// A chain longer than [`Limits::max_xref_sections`] (soft limit) fails like a loop, so the
/// caller falls back to repair. Entries above [`Limits::max_object_number`] are dropped and
/// listed in [`XrefTable::dropped_objects`].
///
/// # Errors
/// [`Error::InvalidStructure`] (or a syntax error) for a section that can't be read; the
/// source's errors ([`Error::DataPending`], [`Error::Source`]), which callers must not
/// repair around.
pub(crate) fn load(source: &Source, start: usize, limits: &Limits, budget: &mut DecodeBudget) -> Result<XrefTable> {
    let mut table = XrefTable::default();
    let mut seen = BTreeSet::new();
    let mut next = Some(start);
    while let Some(offset) = next {
        if !seen.insert(offset) {
            return Err(invalid("/Prev chain loops"));
        }
        if table.sections >= limits.max_xref_sections() {
            return Err(invalid("more /Prev sections than Limits::max_xref_sections"));
        }
        let Section {
            mut entries,
            trailer,
            is_stream,
            free_head,
            skipped_keys,
        } = parse_section(source, offset, limits, budget)?;
        if table.free_head.is_none() {
            table.free_head = free_head;
        }
        table.skipped_trailer_keys |= skipped_keys;
        table.has_xref_stream |= is_stream;
        if let Some(stm) = trailer.get_int("XRefStm").and_then(|o| usize::try_from(o).ok()) {
            // Hybrid file: the classic table's in-use entries win; the stream only fills in
            // numbers the table omits or marks free (compressed objects are typically
            // hidden from pre-1.5 readers that way). A broken XRefStm is ignored.
            match parse_section(source, stm, limits, budget) {
                Ok(Section {
                    entries: stream_entries,
                    ..
                }) => {
                    table.has_xref_stream = true;
                    for (num, entry) in stream_entries {
                        if matches!(entries.get(&num), None | Some(XrefEntry::Free)) {
                            entries.insert(num, entry);
                        }
                    }
                }
                Err(e) if e.must_propagate() => return Err(e),
                Err(_) => table.ignored_xref_streams.push(u64::try_from(stm).unwrap_or(u64::MAX)),
            }
        }
        table.section_objects.push(SectionObjects::of(&entries, limits));
        for (num, entry) in entries {
            if num > limits.max_object_number() {
                table.dropped_objects.insert(num);
                continue;
            }
            table.entries.entry(num).or_insert(entry);
        }
        next = trailer.get_int("Prev").and_then(|p| usize::try_from(p).ok());
        if table.sections == 0 {
            table.trailer = trailer;
            table.newest_is_stream = is_stream;
        }
        table.sections += 1;
        table.section_offsets.push(offset);
    }
    Ok(table)
}

/// What one cross-reference section holds.
#[derive(Debug)]
struct Section {
    entries: BTreeMap<u32, XrefEntry>,
    /// The trailer, or the xref stream's dictionary.
    trailer: Dict,
    is_stream: bool,
    /// Object 0's next-free number, when the section lists object 0 as free.
    free_head: Option<u32>,
    /// Whether the trailer or stream dictionary had something other than a name where a key
    /// belongs, which was skipped (`dictionary-key-skipped`).
    skipped_keys: bool,
}

/// Reads the classic table or the xref stream at `offset`.
fn parse_section(source: &Source, offset: usize, limits: &Limits, budget: &mut DecodeBudget) -> Result<Section> {
    let table = parse_syntax_at(source, offset, Reach::Object, limits, |p, _| {
        let lex = p.lexer();
        lex.skip_whitespace();
        let pos = lex.pos();
        if let Ok(Token::Keyword(b"xref")) = lex.next_token() {
            return parse_table(p).map(|table| Some((table, p.skipped_keys())));
        }
        p.lexer().set_pos(pos);
        Ok(None)
    })??;
    if let Some(((entries, trailer, free_head), skipped_keys)) = table {
        return Ok(Section {
            entries,
            trailer,
            is_stream: false,
            free_head,
            skipped_keys,
        });
    }
    // Decoding happens after the (possibly repeated) windowed parse, so the budget is
    // charged once.
    let parsed = read_indirect(source, offset, limits, &mut |_| None)??;
    let Object::Stream(stream) = parsed.object else {
        return Err(invalid("startxref does not point at an xref"));
    };
    if stream.dict.get_name("Type") != Some(b"XRef") {
        return Err(invalid("stream at startxref is not /Type /XRef"));
    }
    let decoded = filters::decode(&stream.dict, &stream.data, limits, budget)?;
    let (entries, free_head) = parse_stream_entries(&stream.dict, &decoded.data)?;
    Ok(Section {
        entries,
        trailer: stream.dict,
        is_stream: true,
        free_head,
        skipped_keys: parsed.skipped_keys,
    })
}

/// Reads the subsections and the trailer after `xref`, with `parser`'s lexer just past it.
fn parse_table(parser: &mut Parser<'_>) -> Result<(BTreeMap<u32, XrefEntry>, Dict, Option<u32>)> {
    let mut entries = BTreeMap::new();
    let mut head = None;
    let lex = parser.lexer();
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
                        b"f" => {
                            if num == 0 {
                                head = u32::try_from(off).ok();
                            }
                            XrefEntry::Free
                        }
                        _ => return Err(invalid("entry type must be n or f")),
                    };
                    // Within one table, a later subsection for the same number wins.
                    entries.insert(num, entry);
                }
            }
            _ => return Err(invalid("expected subsection or trailer")),
        }
    }
    match parser.object()? {
        Object::Dict(d) => Ok((entries, d, head)),
        _ => Err(invalid("trailer is not a dictionary")),
    }
}

fn read_field(row: &[u8]) -> u64 {
    row.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b))
}

fn parse_stream_entries(dict: &Dict, decoded: &[u8]) -> Result<(BTreeMap<u32, XrefEntry>, Option<u32>)> {
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
    let mut head = None;
    let mut rows = decoded.chunks_exact(row_len);
    let (pairs, _) = index.as_chunks::<2>();
    for [start, count] in pairs {
        let start = u32::try_from(*start).map_err(|_| invalid("bad /Index start"))?;
        let count = u32::try_from(*count).map_err(|_| invalid("bad /Index count"))?;
        for i in 0..count {
            // Rows run out before a bogus count does: the loop is bounded by the data.
            let Some(row) = rows.next() else {
                return Ok((entries, head));
            };
            let (f1, rest) = row.split_at(w1);
            let (f2, f3) = rest.split_at(w2);
            let kind = if w1 == 0 { 1 } else { read_field(f1) };
            let (f2, f3) = (read_field(f2), read_field(f3));
            let Some(num) = start.checked_add(i) else {
                return Ok((entries, head));
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
                kind => {
                    if num == 0 && kind == 0 {
                        head = u32::try_from(f2).ok();
                    }
                    XrefEntry::Free
                }
            };
            entries.insert(num, entry);
        }
    }
    Ok((entries, head))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_default(data: &[u8], start: usize) -> Result<XrefTable> {
        let limits = Limits::default();
        load(
            &Source::from(data.to_vec()),
            start,
            &limits,
            &mut DecodeBudget::new(&limits),
        )
    }

    fn section(data: &[u8]) -> Result<Section> {
        let limits = Limits::default();
        parse_section(
            &Source::from(data.to_vec()),
            0,
            &limits,
            &mut DecodeBudget::new(&limits),
        )
    }

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
        let entry = load_default(&d, table).unwrap().entries.get(&1).copied();
        assert_eq!(entry, Some(XrefEntry::InFile { offset: 100, gen_nr: 0 }));
    }

    #[test]
    fn hybrid_xrefstm_fills_in_entry_the_table_marks_free() {
        let (d, table) = hybrid();
        let entry = load_default(&d, table).unwrap().entries.get(&2).copied();
        assert_eq!(entry, Some(XrefEntry::InStream { stream: 9, index: 1 }));
    }

    #[test]
    fn hybrid_with_broken_xrefstm_records_its_offset() {
        let (mut d, table) = hybrid();
        let at = d.windows(7).position(|w| w == b"7 0 obj").unwrap();
        d[at] = b'x';
        assert_eq!(load_default(&d, table).unwrap().ignored_xref_streams, [9]);
    }

    #[test]
    fn load_drops_entries_above_max_object_number() {
        let (d, table) = hybrid();
        let limits = Limits::default().with_max_object_number(1);
        let loaded = load(&Source::from(d), table, &limits, &mut DecodeBudget::new(&limits)).unwrap();
        assert_eq!(
            (
                loaded.entries.keys().copied().collect::<Vec<_>>(),
                loaded.dropped_objects
            ),
            (vec![0, 1], BTreeSet::from([2]))
        );
    }

    #[test]
    fn find_startxref_reads_last_offset() {
        let source = Source::from(b"startxref\n1\n%%EOF\nstartxref\n42\n%%EOF".to_vec());
        assert_eq!(find_startxref(&source, &Limits::default()), Ok(42));
    }

    #[test]
    fn parse_table_accepts_mixed_eols() {
        let input = b"xref\n0 2\n0000000000 65535 f\r\n0000000017 00000 n \ntrailer\n<< /Size 2 >>";
        let entries = section(input).unwrap().entries;
        assert_eq!(entries.get(&1), Some(&XrefEntry::InFile { offset: 17, gen_nr: 0 }));
    }

    #[test]
    fn parse_table_rejects_huge_count_without_entries() {
        assert!(section(b"xref\n0 4000000000\ntrailer<<>>").is_err());
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
            parse_stream_entries(&dict, &[9]).unwrap().0.get(&0),
            Some(&XrefEntry::InFile { offset: 9, gen_nr: 0 })
        );
    }
}
