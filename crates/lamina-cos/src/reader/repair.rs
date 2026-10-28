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

use std::collections::BTreeMap;

use super::objstm;
use super::xref::{XREF_STREAM_KEYS, XrefEntry, XrefTable};
use lamina_syntax::{Dict, Lexer, ObjRef, Object, Parser, Token, filters, find, is_regular, is_whitespace};

use crate::error::{Error, Result};
use crate::scan::is_boundary;

/// A `num gen obj` header found by scanning: `(num, gen, offset of num)`.
type Header = (u32, u16, usize);

/// Finds every `num gen obj` header. Linear: each byte is walked back over at most once
/// per following `obj` keyword, and runs are delimited by that keyword.
pub(crate) fn scan_object_headers(data: &[u8]) -> Vec<Header> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(p) = find(data, b"obj", from) {
        from = p + 3;
        if !is_boundary(data, p + 3) {
            continue;
        }
        if let Some(header) = header_before(data, p) {
            found.push(header);
        }
    }
    found
}

fn header_before(data: &[u8], obj_kw: usize) -> Option<Header> {
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
    let well_formed = gen_end < obj_kw && gen_start < gen_end && num_end < gen_start && num_start < num_end;
    let starts_token = num_start == 0 || data.get(num_start - 1).is_some_and(|&b| !is_regular(b));
    if !well_formed || !starts_token {
        return None;
    }
    let parse = |s: usize, e: usize| std::str::from_utf8(data.get(s..e)?).ok()?.parse::<u64>().ok();
    let num = u32::try_from(parse(num_start, num_end)?).ok()?;
    let gen_nr = u16::try_from(parse(gen_start, gen_end)?).ok()?;
    Some((num, gen_nr, num_start))
}

fn trailer_dicts(data: &[u8]) -> Vec<(usize, Dict)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = find(data, b"trailer", from) {
        from = p + b"trailer".len();
        if !is_boundary(data, from) || (p > 0 && !is_boundary(data, p - 1)) {
            continue;
        }
        if let Ok(Object::Dict(d)) = Parser::new(data, from).object() {
            out.push((p, d));
        }
    }
    out
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
fn parse_head(data: &[u8], offset: usize) -> Option<Head> {
    let mut parser = Parser::new(data, offset);
    parser.object_header().ok()?;
    let head = parser.object().ok()?;
    let is_stream = matches!(head, Object::Dict(_)) && {
        let mut lex = Lexer::new(data, parser.pos());
        matches!(lex.next_token(), Ok(Token::Keyword(b"stream"))) && find(data, b"endstream", lex.pos()).is_some()
    };
    Some((head, is_stream))
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

/// Decodes the object stream at `offset` and returns its members as candidates.
fn object_stream_members(data: &[u8], offset: usize, stream: u32, limit: usize) -> Vec<(u32, Candidate)> {
    let Ok((_, Object::Stream(s))) = Parser::new(data, offset).indirect_object(&mut |_| None) else {
        return Vec::new();
    };
    let Ok(decoded) = filters::decode(&s.dict, &s.data, limit) else {
        return Vec::new();
    };
    let Ok(objs) = objstm::parse(&s.dict, &decoded) else {
        return Vec::new();
    };
    (0u32..)
        .zip(objs)
        .map(|(index, (member, obj))| {
            let candidate = Candidate {
                entry: XrefEntry::InStream { stream, index },
                position: (offset, index),
                summary: Some(summarize(&obj, false)),
            };
            (member, candidate)
        })
        .collect()
}

/// Rebuilds the cross-reference table by scanning the whole file.
pub(crate) fn rebuild(data: &[u8], limit: usize) -> Result<XrefTable> {
    let headers = scan_object_headers(data);
    if headers.is_empty() {
        return Err(Error::NotPdf("no objects found"));
    }
    // In-file definitions first: the trailers and object streams found among the winners
    // decide which compressed members exist.
    let mut winners: BTreeMap<u32, Candidate> = BTreeMap::new();
    let mut heads: Vec<(u32, usize, Option<Head>)> = Vec::new();
    for &(num, gen_nr, offset) in &headers {
        let head = parse_head(data, offset);
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
        heads.push((num, offset, head));
    }

    let mut trailers = trailer_dicts(data);
    let mut members = Vec::new();
    for (num, offset, head) in heads {
        let Some((Object::Dict(dict), is_stream)) = head else {
            continue;
        };
        let is_winner = matches!(winners.get(&num), Some(c) if c.position == (offset, 0));
        if !is_winner {
            continue;
        }
        match dict.get_name("Type") {
            Some(b"XRef") => trailers.push((offset, dict)),
            Some(b"ObjStm") if is_stream => members.extend(object_stream_members(data, offset, num, limit)),
            _ => {}
        }
    }
    for (member, candidate) in members {
        offer(&mut winners, member, candidate);
    }

    let has_xref_stream = trailers.iter().any(|(_, d)| d.get_name("Type") == Some(b"XRef"));
    trailers.sort_by_key(|(offset, _)| *offset);
    let sections = trailers.len().max(1);
    let mut trailer = Dict::new();
    for (_, dict) in &trailers {
        for (k, v) in dict.iter() {
            trailer.insert(k.clone(), v.clone());
        }
    }
    let trailers_newest_first: Vec<Dict> = trailers.into_iter().rev().map(|(_, d)| d).collect();
    // `/Size` keeps its position and is recomputed below; the other xref keys are stale.
    for key in XREF_STREAM_KEYS.iter().filter(|k| **k != "Size") {
        trailer.remove(key);
    }
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
    let root = trailers_newest_first
        .iter()
        .filter_map(|d| d.get("Root").and_then(Object::as_ref))
        .find(|r| is_root_candidate(*r))
        .or(newest_catalog)
        .ok_or(Error::MissingRoot)?;
    trailer.insert("Root", root);
    let entries: BTreeMap<u32, XrefEntry> = winners.into_iter().map(|(num, c)| (num, c.entry)).collect();
    let max = entries.keys().next_back().copied().unwrap_or(0);
    trailer.insert("Size", i64::from(max) + 1);
    Ok(XrefTable {
        entries,
        trailer,
        sections,
        has_xref_stream,
        newest_is_stream: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_finds_headers_and_skips_endobj() {
        let headers = scan_object_headers(b"1 0 obj 5 endobj\n12 3 obj\n(x) endobj");
        assert_eq!(headers, vec![(1, 0, 0), (12, 3, 17)]);
    }

    #[test]
    fn scan_ignores_obj_inside_longer_tokens() {
        assert_eq!(scan_object_headers(b"x1 0 obj 1 0 objx"), Vec::<Header>::new());
    }

    #[test]
    fn rebuild_adopts_catalog_when_trailer_is_missing() {
        let data = b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n";
        assert_eq!(
            rebuild(data, 1024).unwrap().trailer.get("Root"),
            Some(&Object::Ref(ObjRef::new(1, 0)))
        );
    }

    #[test]
    fn rebuild_keeps_intact_definition_when_later_one_is_unparsable() {
        let data = b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n1 0 obj << /Pages ] endobj\n";
        assert_eq!(
            rebuild(data, 1024).unwrap().entries.get(&1),
            Some(&XrefEntry::InFile { offset: 9, gen_nr: 0 })
        );
    }

    #[test]
    fn rebuild_skips_trailer_root_that_is_not_a_dictionary() {
        let data = b"%PDF-1.4\n1 0 obj << /Type /Catalog >> endobj\n2 0 obj 42 endobj\ntrailer << /Root 2 0 R >>\n";
        assert_eq!(
            rebuild(data, 1024).unwrap().trailer.get("Root"),
            Some(&Object::Ref(ObjRef::new(1, 0)))
        );
    }
}
