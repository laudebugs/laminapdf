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
use super::window::{Reach, View, for_each_chunk, parse_syntax_at, read_indirect, rfind_input, within_read_window};
use super::xref::{XREF_STREAM_KEYS, XrefEntry, XrefTable};
use lamina_syntax::{DecodeBudget, Dict, Limits, ObjRef, Object, Token, filters, find, is_regular, is_whitespace};

use crate::error::{Error, Result};
use crate::scan::is_boundary;
use crate::source::{Fetch, Source};

/// A `num gen obj` header found by scanning: `(num, gen, offset of num)`.
type Header = (u32, u16, usize);

/// Bytes a chunk of the scan needs around its own part: the byte before `trailer` (a keyword
/// boundary), and after an `obj` or `trailer` that starts in the chunk, the keyword and the
/// byte after it.
const CHUNK_AROUND: (usize, usize) = (1, b"trailer".len() + 1);

/// Bytes the backward walk over `num gen` reads first when it runs into the start of a chunk;
/// doubled (reading only the new part) until the walk ends inside them, up to
/// [`Limits::max_read_window`].
const HEADER_BACK: usize = 64;

/// What one pass over the input found: every `num gen obj` header and the head of the object
/// after it, and every `trailer` dictionary, in file order.
struct Found {
    objects: Vec<(Header, Option<ScannedHead>)>,
    trailers: Vec<(usize, Dict)>,
}

/// Walks the input once ([`for_each_chunk`]: in overlapping chunks for a source without an
/// in-memory slice) and collects every object header with its head and every trailer
/// dictionary. Linear: each byte is walked back over at most once per following `obj`
/// keyword, and runs are delimited by that keyword. An `obj` or `trailer` belongs to the
/// chunk it starts in; a header whose `num gen` begins in an earlier chunk is read back from
/// the input.
///
/// # Errors
/// [`Error::DataPending`] with the windows still missing (one round's worth); [`Error::Source`].
fn scan_input(source: &Source, limits: &Limits) -> Result<Found> {
    let mut found = Found {
        objects: Vec::new(),
        trailers: Vec::new(),
    };
    for_each_chunk(source, CHUNK_AROUND, |view, own| {
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
            if let Some(header) = header_ending_at(source, view, p, limits)? {
                let head = if header.0 > limits.max_object_number() {
                    None
                } else {
                    parse_head(source, header.2, limits)?
                };
                found.objects.push((header, head));
            }
        }
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
            let dict = parse_syntax_at(source, view.base + from, Reach::Object, limits, |parser, _| {
                Ok(parser.object()?)
            });
            if let Some(Ok(Object::Dict(d))) = within_read_window(dict)? {
                found.trailers.push((view.base + p, d));
            }
        }
        Ok(())
    })?;
    Ok(found)
}

/// Every `num gen obj` header, in file order (see [`scan_input`]).
#[cfg(test)]
fn scan_object_headers(source: &Source) -> Result<Vec<Header>> {
    let found = scan_input(source, &Limits::default())?;
    Ok(found.objects.into_iter().map(|(header, _)| header).collect())
}

/// The header whose `obj` keyword is at `view.data[obj_kw]`, with its absolute offset; reads
/// further back from the input while the walk over `num gen` reaches the start of what it
/// has. A walk longer than [`Limits::max_read_window`] finds no header.
fn header_ending_at(source: &Source, view: View<'_>, obj_kw: usize, limits: &Limits) -> Result<Option<Header>> {
    let absolute = |(num, gen_nr, offset): Header, base: usize| (num, gen_nr, offset + base);
    if let HeaderScan::Done(header) = header_before(view.data, obj_kw, view.base == 0) {
        return Ok(header.map(|h| absolute(h, view.base)));
    }
    let kw = view.base + obj_kw;
    let end = kw + b"obj".len();
    let mut have = source.read(view.base, end - view.base, Fetch::Exact)?;
    let mut back = HEADER_BACK;
    loop {
        let start = kw.saturating_sub(back);
        if start < have.start() {
            let before = source.read(start, have.start() - start, Fetch::Exact)?;
            have = have.prepend(before);
        }
        if let HeaderScan::Done(header) = header_before(have.bytes(), kw - have.start(), have.start() == 0) {
            return Ok(header.map(|h| absolute(h, have.start())));
        }
        if back >= limits.max_read_window() {
            return Ok(None);
        }
        back = back.saturating_mul(2).min(limits.max_read_window());
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
        return HeaderScan::Done(None);
    }
    let parse = |s: usize, e: usize| std::str::from_utf8(data.get(s..e)?).ok()?.parse::<u64>().ok();
    let header = || {
        let num = u32::try_from(parse(num_start, num_end)?).ok()?;
        let gen_nr = u16::try_from(parse(gen_start, gen_end)?).ok()?;
        Some((num, gen_nr, num_start))
    };
    HeaderScan::Done(header())
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

/// What repair keeps of an object's head (its value without stream data).
#[derive(Debug)]
struct ScannedHead {
    /// The value is a reference.
    is_ref: bool,
    /// The value is a dictionary (and maybe a stream's).
    is_dict: bool,
    /// `/Type /Catalog`.
    catalog: bool,
    /// For a dictionary followed by `stream`: where the data starts. The object is a complete
    /// stream when an `endstream` follows somewhere.
    stream_data: Option<usize>,
    /// The dictionary of an xref or object stream, which repair needs later.
    kept: Option<Dict>,
}

impl ScannedHead {
    /// Whether the object is a complete stream, given the offset of the last `endstream` in
    /// the input: one follows its data.
    fn is_stream(&self, last_endstream: Option<usize>) -> bool {
        self.stream_data
            .is_some_and(|from| last_endstream.is_some_and(|last| last >= from))
    }

    fn summary(&self, last_endstream: Option<usize>) -> Summary {
        Summary {
            root_candidate: self.is_ref || (self.is_dict && !self.is_stream(last_endstream)),
            catalog: self.catalog,
        }
    }
}

/// Parses the head of the indirect object at `offset`: its header and value, but not stream
/// data. A stream whose `endstream` is missing still yields its dictionary, so trailers and
/// catalogs inside broken xref streams are recovered. `None` when it does not parse (or would
/// pass [`Limits::max_read_window`]).
///
/// # Errors
/// The source's errors.
fn parse_head(source: &Source, offset: usize, limits: &Limits) -> Result<Option<ScannedHead>> {
    let head = parse_syntax_at(source, offset, Reach::Object, limits, |parser, base| {
        parser.object_header()?;
        let value = parser.object()?;
        let mut stream_data = None;
        if matches!(value, Object::Dict(_))
            && let Ok(Token::Keyword(b"stream")) = parser.lexer().next_token()
        {
            stream_data = Some(base + parser.pos());
        }
        let dict = value.as_dict();
        let kind = dict.and_then(|d| d.get_name("Type"));
        Ok(ScannedHead {
            is_ref: matches!(value, Object::Ref(_)),
            is_dict: dict.is_some(),
            catalog: kind == Some(b"Catalog"),
            stream_data,
            kept: dict.filter(|_| matches!(kind, Some(b"XRef" | b"ObjStm"))).cloned(),
        })
    });
    Ok(within_read_window(head)?.and_then(Result::ok))
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
    let parsed = within_read_window(read_indirect(source, offset, limits, &mut |_| None))?;
    let Some(Ok(mut parsed)) = parsed else {
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
    let Found { objects, mut trailers } = scan_input(source, limits)?;
    if objects.is_empty() {
        return Err(Error::NotPdf("no objects found"));
    }
    // One search answers "is an `endstream` after this stream's data?" for every head (the
    // scan above has read the whole input, so this does not ask for more).
    let last_endstream = if objects
        .iter()
        .any(|(_, head)| head.as_ref().is_some_and(|h| h.stream_data.is_some()))
    {
        rfind_input(source, b"endstream")?
    } else {
        None
    };
    let mut dropped_objects = BTreeSet::new();
    // In-file definitions first: the trailers and object streams found among the winners
    // decide which compressed members exist.
    let mut winners: BTreeMap<u32, Candidate> = BTreeMap::new();
    let mut heads = Vec::new();
    for ((num, gen_nr, offset), head) in objects {
        if num > limits.max_object_number() {
            dropped_objects.insert(num);
            continue;
        }
        let candidate = Candidate {
            entry: XrefEntry::InFile { offset, gen_nr },
            position: (offset, 0),
            summary: head.as_ref().map(|h| h.summary(last_endstream)),
        };
        offer(&mut winners, num, candidate);
        // Only dictionaries of xref and object streams are needed later.
        if let Some(head) = head
            && head.kept.is_some()
        {
            heads.push((num, gen_nr, offset, head));
        }
    }
    let mut object_streams = Vec::new();
    for (num, gen_nr, offset, head) in heads {
        let is_winner = matches!(winners.get(&num), Some(c) if c.position == (offset, 0));
        let is_stream = head.is_stream(last_endstream);
        let Some(dict) = head.kept.filter(|_| is_winner) else {
            continue;
        };
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
        let found = scan_input(&NoSlice::source(data).0, &Limits::default())
            .unwrap()
            .trailers;
        assert_eq!(found.iter().map(|(at, _)| *at).collect::<Vec<_>>(), [WINDOW - 4]);
    }

    #[test]
    fn a_trailer_keyword_glued_to_the_byte_before_a_chunk_boundary_is_not_one() {
        let data = placed(2 * WINDOW, WINDOW - 1, b"xtrailer << /Root 1 0 R >>");
        let found = scan_input(&NoSlice::source(data).0, &Limits::default())
            .unwrap()
            .trailers;
        assert_eq!(found.len(), 0);
    }

    #[test]
    fn the_walk_back_over_a_header_stops_at_max_read_window() {
        // `5 0`, 300 spaces, then `obj` at the start of the second window.
        let mut text = b"5 0".to_vec();
        text.extend(vec![b' '; 300]);
        text.extend_from_slice(b"obj");
        let at = WINDOW - text.len() + 3;
        let data = placed(2 * WINDOW, at, &text);
        let limits = Limits::default().with_max_read_window(128);
        let found = scan_input(&NoSlice::source(data).0, &limits).unwrap();
        assert_eq!(found.objects.len(), 0);
    }
}
