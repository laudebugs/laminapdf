//! Reader/writer policies agreed with the TypeScript twin after differential fuzzing
//! (see README "Alignment with the TS twin").
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::fixture;
use common::open;
use lamina_cos::{Dict, Error, ObjRef, Object, Warning, Writer};
use lamina_syntax::parse_object;

/// A classic file around arbitrary object bodies, with a correct xref.
fn file_with(header: &[u8], objects: &[&[u8]], trailer: &str) -> Vec<u8> {
    let mut out = header.to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n{trailer}\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

const CATALOG: &[u8] = b"<< /Type /Catalog /Pages 2 0 R >>";

fn replace(bytes: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    assert_eq!(from.len(), to.len(), "same-length replacement keeps offsets valid");
    let at = bytes.windows(from.len()).position(|w| w == from).unwrap();
    [&bytes[..at], to, &bytes[at + from.len()..]].concat()
}

// 1. Missing objects are null; broken page-tree children are skipped; cycles still error.

#[test]
fn page_count_is_zero_when_all_kids_are_missing() {
    let bytes = file_with(
        b"%PDF-1.7\n",
        &[CATALOG, b"<< /Type /Pages /Kids [8 0 R 9 0 R] /Count 2 >>"],
        "<< /Size 3 /Root 1 0 R >>",
    );
    assert_eq!(open(&bytes).unwrap().page_count(), Ok(0));
}

#[test]
fn page_count_skips_missing_and_non_dict_kids() {
    let bytes = file_with(
        b"%PDF-1.7\n",
        &[
            CATALOG,
            b"<< /Type /Pages /Kids [9 0 R 3 0 R 42 (junk)] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R >>",
        ],
        "<< /Size 4 /Root 1 0 R >>",
    );
    assert_eq!(open(&bytes).unwrap().page_count(), Ok(1));
}

#[test]
fn page_count_is_zero_when_pages_root_is_missing() {
    let bytes = file_with(
        b"%PDF-1.7\n",
        &[b"<< /Type /Catalog /Pages 7 0 R >>"],
        "<< /Size 2 /Root 1 0 R >>",
    );
    assert_eq!(open(&bytes).unwrap().page_count(), Ok(0));
}

#[test]
fn page_count_is_zero_when_kids_is_not_an_array() {
    let bytes = file_with(
        b"%PDF-1.7\n",
        &[CATALOG, b"<< /Type /Pages /Kids 5 /Count 1 >>"],
        "<< /Size 3 /Root 1 0 R >>",
    );
    assert_eq!(open(&bytes).unwrap().page_count(), Ok(0));
}

#[test]
fn reference_cycle_among_kids_still_errors() {
    let bytes = file_with(
        b"%PDF-1.7\n",
        &[CATALOG, b"<< /Type /Pages /Kids [3 0 R] >>", b"4 0 R", b"3 0 R"],
        "<< /Size 5 /Root 1 0 R >>",
    );
    assert!(matches!(
        open(&bytes).unwrap().page_count(),
        Err(Error::ReferenceCycle(_))
    ));
}

#[test]
fn missing_object_resolves_to_null() {
    let bytes = fixture("golden/hello.pdf");
    assert_eq!(
        open(&bytes).unwrap().resolve(ObjRef::new(99, 0)).as_deref(),
        Ok(&Object::Null)
    );
}

// 2. The `%PDF-` header is not required.

#[test]
fn file_without_header_still_opens() {
    let bytes = file_with(
        b"",
        &[CATALOG, b"<< /Type /Pages /Kids [3 0 R] >>", b"<< /Type /Page >>"],
        "<< /Size 4 /Root 1 0 R >>",
    );
    assert_eq!(open(&bytes).unwrap().page_count(), Ok(1));
}

#[test]
fn file_without_header_records_a_warning() {
    let bytes = replace(&fixture("golden/hello.pdf"), b"%PDF-1.7", b"%PDX-1.7");
    let doc = open(&bytes).unwrap();
    assert_eq!(
        (doc.header_version(), doc.warnings()),
        (None, vec![Warning::MissingHeader])
    );
}

#[test]
fn corrupt_header_does_not_force_repair() {
    let bytes = replace(&fixture("golden/hello.pdf"), b"%PDF-1.7", b"garbage!");
    assert!(!open(&bytes).unwrap().was_repaired());
}

#[test]
fn header_after_junk_prefix_is_found_and_repaired() {
    let bytes = [vec![b'x'; 200], fixture("golden/hello.pdf")].concat();
    let doc = open(&bytes).unwrap();
    assert_eq!(
        (doc.header_version(), doc.was_repaired(), doc.page_count()),
        (Some("1.7"), true, Ok(1))
    );
}

#[test]
fn header_only_input_is_still_an_error() {
    assert!(open(b"%PDF-").is_err());
}

// 3. Repair recovers object streams and /Root from damaged xref streams and trailers.

fn xrefstm() -> Vec<u8> {
    fixture("corpus/hello-xrefstm.pdf")
}

#[test]
fn repair_with_bad_startxref_indexes_object_stream_members() {
    let bytes = replace(&xrefstm(), b"startxref\n362", b"startxref\n999");
    let doc = open(&bytes).unwrap();
    let font = doc.resolve(ObjRef::new(4, 0)).unwrap();
    assert_eq!(
        (doc.was_repaired(), font.as_dict().and_then(|d| d.get_name("BaseFont"))),
        (true, Some(&b"Helvetica"[..]))
    );
}

#[test]
fn repair_with_bad_startxref_counts_the_page() {
    let bytes = replace(&xrefstm(), b"startxref\n362", b"startxref\n999");
    assert_eq!(open(&bytes).unwrap().page_count(), Ok(1));
}

#[test]
fn repair_survives_corrupted_xref_stream_data() {
    let bytes = xrefstm();
    let at = bytes.windows(9).rposition(|w| w == b"endstream").unwrap() - 10;
    let mut damaged = bytes;
    for b in &mut damaged[at..at + 8] {
        *b ^= 0x5A;
    }
    let doc = open(&damaged).unwrap();
    assert_eq!((doc.was_repaired(), doc.page_count()), (true, Ok(1)));
}

#[test]
fn repair_finds_root_in_xref_stream_whose_stream_is_truncated() {
    // Cut the file inside the xref stream data: no `endstream`, no `startxref`.
    let bytes = xrefstm();
    let at = bytes.windows(9).rposition(|w| w == b"endstream").unwrap() - 10;
    let doc = open(&bytes[..at]).unwrap();
    assert_eq!(doc.trailer().get("Root"), Some(&Object::Ref(ObjRef::new(1, 0))));
}

#[test]
fn repair_adopts_catalog_inside_object_stream_when_root_is_gone() {
    let bytes = replace(&xrefstm(), b"/Root 1 0 R", b"/Xoot 1 0 R");
    let doc = open(&bytes).unwrap();
    assert_eq!((doc.was_repaired(), doc.page_count()), (true, Ok(1)));
}

#[test]
fn repair_prefers_newest_root_that_points_at_an_existing_object() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    bytes.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    bytes.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [] /Count 0 >> endobj\n");
    bytes.extend_from_slice(b"trailer << /Root 1 0 R >>\n");
    bytes.extend_from_slice(b"trailer << /Root 77 0 R >>\n");
    assert_eq!(
        open(&bytes).unwrap().trailer().get("Root"),
        Some(&Object::Ref(ObjRef::new(1, 0)))
    );
}

#[test]
fn repair_takes_newest_root_when_all_are_valid() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    bytes.extend_from_slice(b"1 0 obj << /Type /Catalog >> endobj\n3 0 obj << /Type /Catalog >> endobj\n");
    bytes.extend_from_slice(b"trailer << /Root 1 0 R >>\ntrailer << /Root 3 0 R >>\n");
    assert_eq!(
        open(&bytes).unwrap().trailer().get("Root"),
        Some(&Object::Ref(ObjRef::new(3, 0)))
    );
}

#[test]
fn repaired_revisions_count_trailers_found() {
    let mut bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\n".to_vec();
    bytes.extend_from_slice(b"trailer << /Root 1 0 R >>\ntrailer << /Root 1 0 R >>\n");
    assert_eq!(open(&bytes).unwrap().revisions(), 2);
}

#[test]
fn repaired_revisions_are_at_least_one_without_trailers() {
    let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\n";
    assert_eq!(open(bytes).unwrap().revisions(), 1);
}

// 4. Writer conventions.

fn trailer_of(bytes: &[u8]) -> Vec<u8> {
    let at = bytes.windows(8).position(|w| w == b"trailer\n").unwrap() + 8;
    let end = bytes.windows(10).position(|w| w == b"\nstartxref").unwrap();
    bytes[at..end].to_vec()
}

#[test]
fn trailer_size_is_written_first_when_absent() {
    let mut w = Writer::new(1, 7);
    let root = w.add(Dict::new());
    let bytes = w.finish(&[("Root", root)].into_iter().collect()).unwrap();
    assert_eq!(trailer_of(&bytes), b"<< /Size 2 /Root 1 0 R >>");
}

#[test]
fn trailer_size_keeps_caller_position_with_correct_value() {
    let mut w = Writer::new(1, 7);
    let root = w.add(Dict::new());
    let trailer: Dict = [("Root", Object::from(root)), ("Size", Object::from(999))]
        .into_iter()
        .collect();
    assert_eq!(trailer_of(&w.finish(&trailer).unwrap()), b"<< /Root 1 0 R /Size 2 >>");
}

// 5. Integer range: Rust keeps full i64 (TS rejects beyond ±2^53).

#[test]
fn integers_beyond_two_pow_53_parse_as_integers() {
    assert_eq!(
        parse_object(b"9007199254740993"),
        Ok(Object::Integer(9_007_199_254_740_993))
    );
}

// 6. A stored xref whose `/Root` does not resolve to a dictionary triggers repair (like TS).
//    The single-byte mutations are the 10 cases from the TS differential fuzz where TS found
//    1 page and Rust returned -1 (the xref loaded and its entries looked valid).

/// `(page_count, was_repaired)` after opening `bytes`.
fn pages_and_repaired(bytes: &[u8]) -> (Result<usize, Error>, bool) {
    let doc = open(bytes).unwrap();
    (doc.page_count(), doc.was_repaired())
}

/// Replaces the `nth` (0-based) occurrence of `from` with the same-length `to`.
fn replace_nth(bytes: &[u8], from: &[u8], to: &[u8], nth: usize) -> Vec<u8> {
    assert_eq!(from.len(), to.len(), "same-length replacement keeps offsets valid");
    let at = bytes
        .windows(from.len())
        .enumerate()
        .filter(|(_, w)| *w == from)
        .nth(nth)
        .unwrap()
        .0;
    [&bytes[..at], to, &bytes[at + from.len()..]].concat()
}

#[test]
fn syntax_error_in_newest_catalog_repairs_to_the_intact_older_catalog() {
    let bytes = replace_nth(
        &fixture("corpus/signed.pdf"),
        b"/Type /Catalog\n/Pages 2 0 R",
        b"/Type /Catalog\n/Pages 2 0 ]",
        1,
    );
    assert_eq!(pages_and_repaired(&bytes), (Ok(1), true));
}

#[test]
fn syntax_error_in_newest_catalog_of_tampered_file_repairs() {
    let bytes = replace_nth(
        &fixture("corpus/tampered.pdf"),
        b"/Type /Catalog\n/Pages 2 0 R",
        b"/Type /Catalog\n/Pages 2 0 ]",
        1,
    );
    assert_eq!(pages_and_repaired(&bytes), (Ok(1), true));
}

/// Results for `file` with each single-byte mutation `(offset, value)` from the TS fuzz.
fn mutated_results(file: &str, mutations: &[(usize, u8)]) -> Vec<(Result<usize, Error>, bool)> {
    let original = fixture(&format!("corpus/{file}"));
    mutations
        .iter()
        .map(|&(at, value)| {
            let mut bytes = original.clone();
            bytes[at] = value;
            pages_and_repaired(&bytes)
        })
        .collect()
}

#[test]
fn hello_incremental_with_damaged_newest_xref_repairs_to_one_page() {
    assert_eq!(
        mutated_results("hello-incremental.pdf", &[(781, 68)]),
        vec![(Ok(1), true)]
    );
}

#[test]
fn hello_xrefstm_with_damaged_xref_stream_repairs_to_one_page() {
    let mutations = [(412, 138), (415, 178), (415, 56), (454, 87)];
    assert_eq!(mutated_results("hello-xrefstm.pdf", &mutations), vec![(Ok(1), true); 4]);
}

#[test]
fn pdflib_with_unresolvable_root_repairs_to_one_page() {
    let mutations = [(1064, 41), (1079, 80), (1080, 114)];
    assert_eq!(mutated_results("pdflib.pdf", &mutations), vec![(Ok(1), true); 3]);
}

#[test]
fn signed_and_tampered_with_damaged_catalog_repair_to_one_page() {
    let results = [
        mutated_results("signed.pdf", &[(590, 73)]),
        mutated_results("tampered.pdf", &[(590, 73)]),
    ]
    .concat();
    assert_eq!(results, vec![(Ok(1), true); 2]);
}

#[test]
fn intact_files_with_valid_catalog_are_not_repaired() {
    let repaired: Vec<bool> = ["signed.pdf", "hello-incremental.pdf", "hello-xrefstm.pdf", "pdflib.pdf"]
        .iter()
        .map(|f| open(&fixture(&format!("corpus/{f}"))).unwrap().was_repaired())
        .collect();
    assert_eq!(repaired, vec![false; 4]);
}
