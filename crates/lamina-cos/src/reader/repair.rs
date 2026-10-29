//! Xref reconstruction for files whose `startxref`/xref is missing or wrong.
//!
//! Strategy: scan for `num gen obj` headers and read only each object's head (its value up to
//! `stream`; stream data is never copied, except for object streams, which are decoded one at
//! a time). For each object number the definition that appears **last in the file** wins:
//! an in-file object by its header offset, an object-stream member by its container's offset
//! (then by its index in the container). Definitions that do not parse lose to any that do;
//! an unparsable one is kept only when there is nothing better. Every `trailer` dictionary and
//! every xref-stream dictionary is merged in file order. The newest `/Root` that points at a
//! dictionary wins; if none does, the newest `/Type /Catalog` object (by the same file order)
//! is adopted.

use std::collections::{BTreeMap, BTreeSet};

use super::encrypt::{self, Security};
use super::objstm;
use super::window::{Reach, View, find_input, for_each_chunk, parse_at};
use super::xref::{XREF_STREAM_KEYS, XrefEntry, XrefTable};
use lamina_syntax::{
    DecodeBudget, Dict, Lexer, Limits, ObjRef, Object, Parser, Token, filters, find, is_regular, is_whitespace,
};

use crate::error::{Error, Result};
use crate::scan::is_boundary;
use crate::source::Source;

/// A `num gen obj` header found by scanning: `(num, gen, offset of num)`.
type Header = (u32, u16, usize);

/// Bytes a chunk of the scan needs past its own end: an `obj` that starts in the chunk, and
/// the byte after it (a keyword boundary).
const OBJ_AHEAD: usize = b"obj".len() + 1;

/// Bytes the backward walk over `num gen` asks for first when it runs into the start of a
/// chunk; doubled until the walk ends inside them.
const HEADER_BACK: usize = 64;

/// Finds every `num gen obj` header, in file order. Linear: each byte is walked back over at
/// most once per following `obj` keyword, and runs are delimited by that keyword. A source
/// without an in-memory slice is scanned in overlapping chunks ([`for_each_chunk`]): an `obj`
/// belongs to the chunk it starts in, and a header whose `num gen` begins in an earlier chunk
/// is read back from the input.
///
/// # Errors
/// [`Error::DataPending`] with every window still missing; [`Error::Source`].
pub(crate) fn scan_object_headers(source: &Source) -> Result<Vec<Header>> {
    let mut found = Vec::new();
    for_each_chunk(source, (0, OBJ_AHEAD), |view, own| {
        let data = view.data;
        let mut from = own.start;
        while let Some(p) = find(data, b"obj", from) {
            if p >= own.end {
                break;
            }
            from = p + 3;
            if !is_boundary(data, p + 3) {
                continue;
            }
            if let Some(header) = header_ending_at(source, view, p)? {
                found.push(header);
            }
        }
        Ok(())
    })?;
    Ok(found)
}

/// The header whose `obj` keyword is at `view.data[obj_kw]`, with its absolute offset; reads
/// further back from the input while the walk over `num gen` reaches the start of the view.
fn header_ending_at(source: &Source, view: View<'_>, obj_kw: usize) -> Result<Option<Header>> {
    let absolute = |(num, gen_nr, offset): Header, base: usize| (num, gen_nr, offset + base);
    if let HeaderScan::Done(header) = header_before(view.data, obj_kw, view.base == 0) {
        return Ok(header.map(|h| absolute(h, view.base)));
    }
    let kw = view.base + obj_kw;
    let mut back = HEADER_BACK;
    loop {
        let start = kw.saturating_sub(back);
        let window = source.exact(start, kw + b"obj".len() - start)?;
        if let HeaderScan::Done(header) = header_before(window.bytes(), kw - window.start(), window.start() == 0) {
            return Ok(header.map(|h| absolute(h, window.start())));
        }
        back = back.saturating_mul(2);
    }
}

/// What [`header_before`] found.
enum HeaderScan {
    /// The header, if the bytes before `obj` form one.
    Done(Option<Header>),
    /// The walk back reached the start of the data, which is not the start of the input.
    NeedsMore,
}

/// Walks back from the `obj` keyword at `obj_kw` over `num gen`. `at_input_start` says whether
/// `data[0]` is the first byte of the input (otherwise the bytes before it are unknown).
fn header_before(data: &[u8], obj_kw: usize, at_input_start: bool) -> HeaderScan {
    let found = |header| HeaderScan::Done(header);
    let back_while = |mut j: usize, pred: fn(u8) -> bool| {
        while j > 0 && data.get(j - 1).copied().is_some_and(pred) {
            j -= 1;
        }
        j
    };
    let gen_end = back_while(obj_kw, is_whitespace);
    let gen_start = back_while(gen_end, |b| b.is_ascii_digit());
    let num_end = back_while(gen_start, is_whitespace);
    let num_start = back_while(num_end, |b| b.is_ascii_digit());
    // Every walk stops at index 0 at the latest, and each starts where the previous ended,
    // so reaching the start of the data shows as `num_start == 0`.
    if num_start == 0 && !at_input_start {
        return HeaderScan::NeedsMore;
    }
    let well_formed = gen_end < obj_kw && gen_start < gen_end && num_end < gen_start && num_start < num_end;
    let starts_token = num_start == 0 || data.get(num_start - 1).is_some_and(|&b| !is_regular(b));
    if !well_formed || !starts_token {
        return found(None);
    }
    let parse = |s: usize, e: usize| std::str::from_utf8(data.get(s..e)?).ok()?.parse::<u64>().ok();
    let header = || {
        let num = u32::try_from(parse(num_start, num_end)?).ok()?;
        let gen_nr = u16::try_from(parse(gen_start, gen_end)?).ok()?;
        Some((num, gen_nr, num_start))
    };
    found(header())
}

/// Bytes a chunk of the trailer scan needs around its own part: the byte before `trailer`
/// and the keyword plus the byte after it (keyword boundaries).
const TRAILER_AROUND: (usize, usize) = (1, b"trailer".len() + 1);

/// Every `trailer` keyword followed by a dictionary, with the keyword's offset, in file order.
///
/// # Errors
/// As [`scan_object_headers`].
fn trailer_dicts(source: &Source, limits: &Limits) -> Result<Vec<(usize, Dict)>> {
    let mut out = Vec::new();
    for_each_chunk(source, TRAILER_AROUND, |view, own| {
        let data = view.data;
        let mut from = own.start;
        while let Some(p) = find(data, b"trailer", from) {
            if p >= own.end {
                break;
            }
            from = p + b"trailer".len();
            // `p` is never 0 here unless it is the first byte of the input: chunks after the
            // first start one byte before their own part.
            if !is_boundary(data, from) || p.checked_sub(1).is_some_and(|q| !is_boundary(data, q)) {
                continue;
            }
            let dict = parse_at(source, view.base + from, Reach::Object, |v| {
                let mut parser = Parser::new(v.data, v.pos).with_limits(limits);
                let dict = match parser.object() {
                    Ok(Object::Dict(d)) => Some(d),
                    _ => None,
                };
                (dict, parser.reached_end())
            })?;
            if let Some(d) = dict {
                out.push((view.base + p, d));
            }
        }
        Ok(())
    })?;
    Ok(out)
}

/// What repair needs to know about one definition, without keeping the object itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Summary {
    /// A dictionary (not a stream) or a reference: acceptable as a `/Root` target (a ref→ref
    /// chain is checked on resolve, so cycles stay `ReferenceCycle` errors).
    root_candidate: bool,
    /// `/Type /Catalog`.
    catalog: bool,
}

/// A definition of one object number found by the scan.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    entry: XrefEntry,
    /// File order: the header offset, or the container's offset and the member index.
    position: (usize, u32),
    /// `None` when the definition does not parse.
    summary: Option<Summary>,
}

impl Candidate {
    /// Whether `self` should replace `current`: a parsed definition beats an unparsable one,
    /// and otherwise the later one in the file wins.
    fn beats(&self, current: &Self) -> bool {
        match (self.summary.is_some(), current.summary.is_some()) {
            (true, false) => true,
            (false, true) => false,
            _ => self.position > current.position,
        }
    }
}

/// An object's value (without stream data) and whether it is a complete stream.
type Head = (Object, bool);

/// Parses the head of the indirect object at `offset`: its header and value, but not stream
/// data. Returns the value and whether it is a complete stream (a dictionary followed by
/// `stream` and, somewhere later, `endstream`). A stream whose `endstream` is missing still
/// yields its dictionary, so trailers and catalogs inside broken xref streams are recovered.
///
/// # Errors
/// The source's errors.
fn parse_head(source: &Source, offset: usize, limits: &Limits) -> Result<Option<Head>> {
    // The value, and for a dictionary followed by `stream`, the offset after that keyword.
    let head = parse_at(source, offset, Reach::Object, |v| {
        let mut parser = Parser::new(v.data, v.pos).with_limits(limits);
        let head = parser.object_header().and_then(|_| parser.object());
        let mut reached_end = parser.reached_end();
        let head = head.ok().map(|head| {
            let mut stream_data = None;
            if matches!(head, Object::Dict(_)) {
                let mut lex = Lexer::new(v.data, parser.pos());
                if matches!(lex.next_token(), Ok(Token::Keyword(b"stream"))) {
                    stream_data = Some(v.base + lex.pos());
                }
                reached_end |= lex.reached_end();
            }
            (head, stream_data)
        });
        (head, reached_end)
    })?;
    let Some((head, stream_data)) = head else {
        return Ok(None);
    };
    let is_stream = match stream_data {
        Some(from) => find_input(source, b"endstream", from)?.is_some(),
        None => false,
    };
    Ok(Some((head, is_stream)))
}

fn summarize(obj: &Object, is_stream: bool) -> Summary {
    Summary {
        root_candidate: matches!(obj, Object::Ref(_)) || (matches!(obj, Object::Dict(_)) && !is_stream),
        catalog: obj.as_dict().and_then(|d| d.get_name("Type")) == Some(b"Catalog"),
    }
}

/// Keeps `candidate` for its number if it beats the current one.
fn offer(winners: &mut BTreeMap<u32, Candidate>, num: u32, candidate: Candidate) {
    match winners.get(&num) {
        Some(current) if !candidate.beats(current) => {}
        _ => {
            winners.insert(num, candidate);
        }
    }
}

/// Decodes the object stream at `offset` and returns its members as candidates. A stream that
/// can't be decoded (including one over a decode limit) contributes nothing. With `security`,
/// the container is decrypted first; `malformed` gets its number when its AES data was
/// malformed (raised as `aes-padding-invalid` by the caller).
///
/// # Errors
/// The source's errors.
fn object_stream_members(
    source: &Source,
    (offset, stream, gen_nr): (usize, u32, u16),
    limits: &Limits,
    budget: &mut DecodeBudget,
    (security, malformed): (Option<&Security>, &mut Vec<u32>),
) -> Result<Vec<(u32, Candidate)>> {
    let parsed = parse_at(source, offset, Reach::Object, |v| {
        let mut parser = Parser::new(v.data, v.pos).with_limits(limits);
        let parsed = parser.indirect_object(&mut |_| None);
        (parsed, parser.reached_end())
    })?;
    let Ok(mut parsed) = parsed else {
        return Ok(Vec::new());
    };
    if let Some(security) = security
        && !encrypt::decrypt_for_repair(security, &mut parsed.object, stream, gen_nr)
    {
        malformed.push(stream);
    }
    let Object::Stream(s) = parsed.object else {
        return Ok(Vec::new());
    };
    let mut params = s.dict.clone();
    if security.is_some() {
        encrypt::strip_crypt_filter(&mut params);
    }
    let Ok(decoded) = filters::decode(&params, &s.data, limits, budget) else {
        return Ok(Vec::new());
    };
    let Ok(objs) = objstm::parse(&s.dict, &decoded.data, limits) else {
        return Ok(Vec::new());
    };
    Ok((0u32..)
        .zip(objs)
        .map(|(index, (member, obj))| {
            let candidate = Candidate {
                entry: XrefEntry::InStream { stream, index },
                position: (offset, index),
                // A member that does not parse loses to any definition that does.
                summary: obj.as_ref().ok().map(|obj| summarize(obj, false)),
            };
            (member, candidate)
        })
        .collect())
}

/// The first phase of repair: every in-file definition, every trailer and xref-stream
/// dictionary, and the object streams among the winners, whose members are read in the second
/// phase ([`Scan::finish`]). Between the two the reader can read `/Encrypt` (never stored in an
/// object stream, ISO 32000-1 §7.5.7) and authenticate, so an encrypted file's object streams
/// are decrypted, and the whole file is scanned only once.
pub(crate) struct Scan {
    winners: BTreeMap<u32, Candidate>,
    dropped_objects: BTreeSet<u32>,
    /// Trailers and xref-stream dictionaries with their offsets, in no particular order.
    trailers: Vec<(usize, Dict)>,
    /// Winning object streams: `(offset, number, generation)`.
    object_streams: Vec<(usize, u32, u16)>,
}

/// Scans the whole file (phase 1 of repair). Definitions above [`Limits::max_object_number`]
/// are dropped and listed in [`XrefTable::dropped_objects`].
///
/// # Errors
/// [`Error::NotPdf`] when there is no `num gen obj` header at all; the source's errors
/// (repair needs every window of the input).
pub(crate) fn scan(source: &Source, limits: &Limits) -> Result<Scan> {
    let headers = scan_object_headers(source)?;
    if headers.is_empty() {
        return Err(Error::NotPdf("no objects found"));
    }
    let mut dropped_objects = BTreeSet::new();
    // In-file definitions first: the trailers and object streams found among the winners
    // decide which compressed members exist.
    let mut winners: BTreeMap<u32, Candidate> = BTreeMap::new();
    let mut heads: Vec<(u32, u16, usize, Option<Head>)> = Vec::new();
    for &(num, gen_nr, offset) in &headers {
        if num > limits.max_object_number() {
            dropped_objects.insert(num);
            continue;
        }
        let head = parse_head(source, offset, limits)?;
        let candidate = Candidate {
            entry: XrefEntry::InFile { offset, gen_nr },
            position: (offset, 0),
            summary: head.as_ref().map(|(obj, is_stream)| summarize(obj, *is_stream)),
        };
        offer(&mut winners, num, candidate);
        // Only dictionaries of xref and object streams are needed later; drop the rest now.
        let head = head.filter(|(obj, _)| {
            matches!(
                obj.as_dict().and_then(|d| d.get_name("Type")),
                Some(b"XRef" | b"ObjStm")
            )
        });
        heads.push((num, gen_nr, offset, head));
    }
    let mut trailers = trailer_dicts(source, limits)?;
    let mut object_streams = Vec::new();
    for (num, gen_nr, offset, head) in heads {
        let Some((Object::Dict(dict), is_stream)) = head else {
            continue;
        };
        let is_winner = matches!(winners.get(&num), Some(c) if c.position == (offset, 0));
        if !is_winner {
            continue;
        }
        match dict.get_name("Type") {
            Some(b"XRef") => trailers.push((offset, dict)),
            Some(b"ObjStm") if is_stream => object_streams.push((offset, num, gen_nr)),
            _ => {}
        }
    }
    trailers.sort_by_key(|(offset, _)| *offset);
    Ok(Scan {
        winners,
        dropped_objects,
        trailers,
        object_streams,
    })
}

impl Scan {
    /// Every trailer and xref-stream dictionary merged in file order, without the stale xref
    /// keys (`/Size` is recomputed by [`Scan::finish`]).
    fn merged_trailer(&self) -> Dict {
        let mut trailer = Dict::new();
        for (_, dict) in &self.trailers {
            for (k, v) in dict.iter() {
                trailer.insert(k.clone(), v.clone());
            }
        }
        for key in XREF_STREAM_KEYS.iter().filter(|k| **k != "Size") {
            trailer.remove(key);
        }
        trailer
    }

    /// The in-file objects and the merged trailer: enough to read `/Encrypt` and the `/ID`
    /// before object-stream members are known. Not a usable document table (`/Root` may be
    /// missing).
    pub(crate) fn in_file_table(&self) -> XrefTable {
        XrefTable {
            entries: self.winners.iter().map(|(num, c)| (*num, c.entry)).collect(),
            trailer: self.merged_trailer(),
            sections: self.trailers.len().max(1),
            has_xref_stream: false,
            newest_is_stream: false,
            ignored_xref_streams: Vec::new(),
            dropped_objects: BTreeSet::new(),
        }
    }

    /// Phase 2: reads the object streams' members (decrypting them with `security`), picks
    /// `/Root` and builds the table. The returned list names object streams whose AES data was
    /// malformed.
    ///
    /// # Errors
    /// [`Error::MissingRoot`] when neither a trailer `/Root` nor a `/Type /Catalog` is found;
    /// the source's errors.
    pub(crate) fn finish(
        mut self,
        source: &Source,
        limits: &Limits,
        budget: &mut DecodeBudget,
        security: Option<&Security>,
    ) -> Result<(XrefTable, Vec<u32>)> {
        let mut malformed = Vec::new();
        let mut members = Vec::new();
        for &stream in &self.object_streams {
            members.extend(object_stream_members(
                source,
                stream,
                limits,
                budget,
                (security, &mut malformed),
            )?);
        }
        for (member, candidate) in members {
            if member > limits.max_object_number() {
                self.dropped_objects.insert(member);
            } else {
                offer(&mut self.winners, member, candidate);
            }
        }
        let mut trailer = self.merged_trailer();
        let has_xref_stream = self.trailers.iter().any(|(_, d)| d.get_name("Type") == Some(b"XRef"));
        let sections = self.trailers.len().max(1);
        let winners = self.winners;
        // Newest trailer/xref-stream `/Root` that points at a dictionary we actually found wins;
        // otherwise adopt the newest `/Type /Catalog` object.
        let is_root_candidate = |r: ObjRef| {
            winners
                .get(&r.num)
                .is_some_and(|c| c.entry.matches_gen(r.gen_nr) && c.summary.is_some_and(|s| s.root_candidate))
        };
        let newest_catalog = winners
            .iter()
            .filter(|(_, c)| c.summary.is_some_and(|s| s.catalog))
            .max_by_key(|(_, c)| c.position)
            .map(|(&num, c)| ObjRef::new(num, c.entry.gen_nr()));
        let root = self
            .trailers
            .iter()
            .rev()
            .filter_map(|(_, d)| d.get("Root").and_then(Object::as_ref))
            .find(|r| is_root_candidate(*r))
            .or(newest_catalog)
            .ok_or(Error::MissingRoot)?;
        trailer.insert("Root", root);
        let entries: BTreeMap<u32, XrefEntry> = winners.into_iter().map(|(num, c)| (num, c.entry)).collect();
        let max = entries.keys().next_back().copied().unwrap_or(0);
        trailer.insert("Size", i64::from(max) + 1);
        let table = XrefTable {
            entries,
            trailer,
            sections,
            has_xref_stream,
            newest_is_stream: false,
            ignored_xref_streams: Vec::new(),
            dropped_objects: self.dropped_objects,
        };
        Ok((table, malformed))
    }
}

/// Rebuilds the cross-reference table of an unencrypted file by scanning ([`scan`], then
/// [`Scan::finish`] without decryption).
#[cfg(test)]
fn rebuild(data: &[u8], limits: &Limits, budget: &mut DecodeBudget) -> Result<XrefTable> {
    let source = Source::from(data.to_vec());
    Ok(scan(&source, limits)?.finish(&source, limits, budget, None)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rebuild_default(data: &[u8]) -> Result<XrefTable> {
        let limits = Limits::default();
        rebuild(data, &limits, &mut DecodeBudget::new(&limits))
    }

    fn headers(data: &[u8]) -> Vec<Header> {
        scan_object_headers(&Source::from(data.to_vec())).unwrap()
    }

    #[test]
    fn scan_finds_headers_and_skips_endobj() {
        assert_eq!(
            headers(b"1 0 obj 5 endobj\n12 3 obj\n(x) endobj"),
            vec![(1, 0, 0), (12, 3, 17)]
        );
    }

    #[test]
    fn scan_ignores_obj_inside_longer_tokens() {
        assert_eq!(headers(b"x1 0 obj 1 0 objx"), Vec::<Header>::new());
    }

    #[test]
    fn rebuild_adopts_catalog_when_trailer_is_missing() {
        let data = b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n";
        assert_eq!(
            rebuild_default(data).unwrap().trailer.get("Root"),
            Some(&Object::Ref(ObjRef::new(1, 0)))
        );
    }

    #[test]
    fn rebuild_keeps_intact_definition_when_later_one_is_unparsable() {
        let data = b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n1 0 obj << /Pages ] endobj\n";
        assert_eq!(
            rebuild_default(data).unwrap().entries.get(&1),
            Some(&XrefEntry::InFile { offset: 9, gen_nr: 0 })
        );
    }

    #[test]
    fn rebuild_skips_trailer_root_that_is_not_a_dictionary() {
        let data = b"%PDF-1.4\n1 0 obj << /Type /Catalog >> endobj\n2 0 obj 42 endobj\ntrailer << /Root 2 0 R >>\n";
        assert_eq!(
            rebuild_default(data).unwrap().trailer.get("Root"),
            Some(&Object::Ref(ObjRef::new(1, 0)))
        );
    }
}

#[cfg(test)]
mod limit_tests {
    use super::*;

    #[test]
    fn rebuild_drops_definitions_above_max_object_number() {
        let data = b"%PDF-1.4\n1 0 obj << /Type /Catalog >> endobj\n9 0 obj 42 endobj\n";
        let limits = Limits::default().with_max_object_number(8);
        let table = rebuild(data, &limits, &mut DecodeBudget::new(&limits)).unwrap();
        assert_eq!(
            (table.entries.keys().copied().collect::<Vec<_>>(), table.dropped_objects),
            (vec![1], BTreeSet::from([9]))
        );
    }
}

#[cfg(test)]
mod chunk_tests {
    use super::*;
    use crate::source::WINDOW;
    use crate::source::tests::NoSlice;

    /// Spaces with `text` placed so that it starts at `at`.
    fn placed(len: usize, at: usize, text: &[u8]) -> Vec<u8> {
        let mut data = vec![b' '; len];
        data[at..at + text.len()].copy_from_slice(text);
        data
    }

    fn windowed_headers(data: Vec<u8>) -> Vec<Header> {
        scan_object_headers(&NoSlice::source(data).0).unwrap()
    }

    #[test]
    fn an_obj_keyword_straddling_a_chunk_boundary_is_found() {
        // `ob` ends the first window, `j` starts the second.
        let data = placed(2 * WINDOW, WINDOW - 6, b"12 0 obj");
        assert_eq!(windowed_headers(data), vec![(12, 0, WINDOW - 6)]);
    }

    #[test]
    fn a_header_whose_number_is_in_the_previous_chunk_is_found() {
        // `7 ` ends the first window; `0 obj` starts the second.
        let data = placed(2 * WINDOW, WINDOW - 2, b"7 0 obj");
        assert_eq!(windowed_headers(data), vec![(7, 0, WINDOW - 2)]);
    }

    #[test]
    fn a_header_preceded_by_a_long_white_space_run_across_chunks_is_found() {
        // `obj` at the start of the second window, `5 0` and 300 spaces before it.
        let mut text = b"5 0".to_vec();
        text.extend(vec![b' '; 300]);
        text.extend_from_slice(b"obj");
        let at = WINDOW - text.len() + 3;
        let data = placed(2 * WINDOW, at, &text);
        assert_eq!(windowed_headers(data), vec![(5, 0, at)]);
    }

    #[test]
    fn a_number_glued_to_a_regular_byte_across_chunks_is_not_a_header() {
        let data = placed(2 * WINDOW, WINDOW - 3, b"x12 0 obj");
        assert_eq!(windowed_headers(data), Vec::<Header>::new());
    }

    #[test]
    fn chunked_and_in_memory_scans_agree_on_headers_at_every_offset_near_a_boundary() {
        let mut failures = Vec::new();
        for at in WINDOW - 12..WINDOW + 2 {
            let data = placed(2 * WINDOW, at, b"34 5 obj");
            let in_memory = scan_object_headers(&Source::from(data.clone())).unwrap();
            let windowed = windowed_headers(data);
            if windowed != in_memory {
                failures.push((at, windowed, in_memory));
            }
        }
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn a_trailer_keyword_straddling_a_chunk_boundary_is_found() {
        let data = placed(2 * WINDOW, WINDOW - 4, b"trailer << /Root 1 0 R >>");
        let found = trailer_dicts(&NoSlice::source(data).0, &Limits::default()).unwrap();
        assert_eq!(found.iter().map(|(at, _)| *at).collect::<Vec<_>>(), [WINDOW - 4]);
    }

    #[test]
    fn a_trailer_keyword_glued_to_the_byte_before_a_chunk_boundary_is_not_one() {
        let data = placed(2 * WINDOW, WINDOW - 1, b"xtrailer << /Root 1 0 R >>");
        let found = trailer_dicts(&NoSlice::source(data).0, &Limits::default()).unwrap();
        assert_eq!(found.len(), 0);
    }
}
