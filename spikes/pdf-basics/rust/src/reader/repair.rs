//! Xref reconstruction for files whose `startxref`/xref is missing or wrong.
//!
//! Strategy: scan for `num gen obj` headers (last definition wins), parse each object once,
//! merge every `trailer` dictionary and every xref-stream dictionary in file order, and add
//! the members of any object streams found. If no `/Root` survives, adopt the last
//! `/Type /Catalog` object.

use std::collections::BTreeMap;

use super::objstm;
use super::xref::{XrefEntry, XrefTable};
use crate::error::{Error, Result};
use crate::filters;
use crate::lexer::{is_regular, is_whitespace};
use crate::object::{Dict, ObjRef, Object};
use crate::parser::{Parser, find, is_boundary};

/// Keys meaningless once the xref has been rebuilt from scratch.
const STALE_TRAILER_KEYS: &[&str] = &[
    "Prev",
    "XRefStm",
    "Type",
    "W",
    "Index",
    "Filter",
    "DecodeParms",
    "Length",
];

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

/// Parses the indirect object at `offset`; if that fails (e.g. a stream whose data or
/// `endstream` is damaged), still returns its leading dictionary so trailers and catalogs
/// inside broken xref streams can be recovered.
fn parse_loose(data: &[u8], offset: usize) -> Option<Object> {
    if let Ok((_, obj)) = Parser::new(data, offset).indirect_object(&mut |_| None) {
        return Some(obj);
    }
    let mut parser = Parser::new(data, offset);
    parser.object_header().ok()?;
    parser.object().ok().filter(|o| matches!(o, Object::Dict(_)))
}

/// Rebuilds the cross-reference table by scanning the whole file.
pub(crate) fn rebuild(data: &[u8], limit: usize) -> Result<XrefTable> {
    let headers = scan_object_headers(data);
    if headers.is_empty() {
        return Err(Error::NotPdf("no objects found"));
    }
    let mut entries: BTreeMap<u32, XrefEntry> = BTreeMap::new();
    for &(num, gen_nr, offset) in &headers {
        entries.insert(num, XrefEntry::InFile { offset, gen_nr });
    }

    let mut trailers = trailer_dicts(data);
    let mut catalog: Option<ObjRef> = None;
    let mut members: Vec<(u32, u32, u32)> = Vec::new();
    for (&num, entry) in &entries {
        let XrefEntry::InFile { offset, gen_nr } = *entry else {
            continue;
        };
        let Some(obj) = parse_loose(data, offset) else { continue };
        let Some(dict) = obj.as_dict() else { continue };
        match dict.get_name("Type") {
            Some(b"Catalog") => catalog = Some(ObjRef::new(num, gen_nr)),
            Some(b"XRef") => trailers.push((offset, dict.clone())),
            Some(b"ObjStm") => {
                if let Object::Stream(s) = &obj
                    && let Ok(decoded) = filters::decode(&s.dict, &s.data, limit)
                    && let Ok(objs) = objstm::parse(&s.dict, &decoded)
                {
                    for (index, (member, member_obj)) in (0u32..).zip(objs) {
                        if member_obj.as_dict().and_then(|d| d.get_name("Type")) == Some(b"Catalog") {
                            catalog.get_or_insert(ObjRef::new(member, 0));
                        }
                        members.push((member, num, index));
                    }
                }
            }
            _ => {}
        }
    }
    for (member, stream, index) in members {
        // Objects defined directly in the file take precedence over compressed copies.
        entries.entry(member).or_insert(XrefEntry::InStream { stream, index });
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
    for key in STALE_TRAILER_KEYS {
        trailer.remove(key);
    }
    // Newest trailer/xref-stream `/Root` that points at an object we actually found wins;
    // otherwise adopt a `/Type /Catalog` object.
    let root = trailers_newest_first
        .iter()
        .filter_map(|d| d.get("Root").and_then(Object::as_ref))
        .find(|r| entries.contains_key(&r.num))
        .or(catalog)
        .ok_or(Error::MissingRoot)?;
    trailer.insert("Root", root);
    let max = entries.keys().next_back().copied().unwrap_or(0);
    trailer.insert("Size", i64::from(max) + 1);
    Ok(XrefTable {
        entries,
        trailer,
        sections,
        has_xref_stream,
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
        assert!(scan_object_headers(b"x1 0 obj 1 0 objx").is_empty());
    }

    #[test]
    fn rebuild_adopts_catalog_when_trailer_is_missing() {
        let data = b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n";
        assert_eq!(
            rebuild(data, 1024).unwrap().trailer.get("Root"),
            Some(&Object::Ref(ObjRef::new(1, 0)))
        );
    }
}
