//! ADR 0003 §2: every warning code the reader raises, deduplication, and strict mode (each
//! warning becomes `Error::Strict`).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use common::{classic, corpus, fixture, open, open_with, stream_body};
use laminapdf_cos::{Document, Error, ObjRef, OpenOptions, Warning};
use laminapdf_syntax::filters::flate_encode;

fn strict() -> OpenOptions {
    OpenOptions::default().with_strict(true)
}

fn codes(doc: &Document) -> Vec<&'static str> {
    doc.warnings().iter().map(Warning::code).collect()
}

fn strict_code<T>(result: laminapdf_cos::Result<T>) -> Option<&'static str> {
    match result.err()? {
        Error::Strict { warning } => Some(warning.code()),
        _ => None,
    }
}

/// Catalog 1, pages 2 (`pages` body), then `extra` objects from 3 on.
fn file(pages: &[u8], extra: &[&[u8]]) -> Vec<u8> {
    let mut objects: Vec<&[u8]> = vec![b"<< /Type /Catalog /Pages 2 0 R >>", pages];
    objects.extend_from_slice(extra);
    classic(&objects, &format!("<< /Size {} /Root 1 0 R >>", objects.len() + 1))
}

fn one_page() -> Vec<u8> {
    file(
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[b"<< /Type /Page /Parent 2 0 R >>"],
    )
}

mod missing_header {
    use super::*;

    fn headerless() -> Vec<u8> {
        let bytes = one_page();
        let mut out = b"%XXX-1.7".to_vec();
        out.extend_from_slice(bytes.get(8..).unwrap());
        out
    }

    #[test]
    fn is_raised_at_open() {
        assert_eq!(codes(&open(&headerless()).unwrap()), ["missing-header"]);
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        assert_eq!(strict_code(open_with(&headerless(), strict())), Some("missing-header"));
    }
}

mod xref_repaired {
    use super::*;

    #[test]
    fn is_raised_when_the_xref_is_rebuilt() {
        assert_eq!(
            codes(&open(&fixture("corpus/hello-broken.pdf")).unwrap()),
            ["xref-repaired"]
        );
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        assert_eq!(
            strict_code(open_with(&fixture("corpus/hello-broken.pdf"), strict())),
            Some("xref-repaired")
        );
    }
}

mod xrefstm_ignored {
    use super::*;

    fn broken_xrefstm() -> Vec<u8> {
        classic(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>",
                b"<< /Type /Pages /Kids [] /Count 0 >>",
            ],
            "<< /Size 3 /Root 1 0 R /XRefStm 9 >>",
        )
    }

    #[test]
    fn is_raised_with_the_offset_of_an_unreadable_xrefstm() {
        let doc = open(&broken_xrefstm()).unwrap();
        assert_eq!(
            (doc.was_repaired(), doc.warnings()),
            (false, vec![Warning::XrefStmIgnored { offset: 9 }])
        );
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        assert_eq!(
            strict_code(open_with(&broken_xrefstm(), strict())),
            Some("xrefstm-ignored")
        );
    }
}

mod stream_length_recovered {
    use super::*;

    fn wrong_length() -> Vec<u8> {
        file(
            b"<< /Type /Pages /Kids [] /Count 0 >>",
            &[b"<< /Length 999 >>\nstream\nabc\nendstream"],
        )
    }

    #[test]
    fn is_raised_lazily_when_the_stream_is_resolved() {
        let doc = open(&wrong_length()).unwrap();
        let before = doc.warnings();
        doc.resolve(ObjRef::new(3, 0)).unwrap();
        assert_eq!(
            (before, doc.warnings()),
            (vec![], vec![Warning::StreamLengthRecovered { object: 3 }])
        );
    }

    #[test]
    fn is_raised_once_per_object() {
        let doc = open(&wrong_length()).unwrap();
        for _ in 0..3 {
            doc.resolve(ObjRef::new(3, 0)).unwrap();
        }
        assert_eq!(codes(&doc), ["stream-length-recovered"]);
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        let doc = open_with(&wrong_length(), strict()).unwrap();
        assert_eq!(
            strict_code(doc.resolve(ObjRef::new(3, 0))),
            Some("stream-length-recovered")
        );
    }

    #[test]
    fn is_not_cached_as_success_in_strict_mode() {
        let doc = open_with(&wrong_length(), strict()).unwrap();
        drop(doc.resolve(ObjRef::new(3, 0)));
        assert_eq!(
            strict_code(doc.resolve(ObjRef::new(3, 0))),
            Some("stream-length-recovered")
        );
    }
}

mod dictionary_key_skipped {
    use super::*;

    /// A page whose dictionary has a stray `)` and keyword where keys belong.
    fn stray_tokens() -> Vec<u8> {
        file(
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            &[b"<< /Type /Page ) /Parent 2 0 R Rotate 90 /Rotate 180 >>"],
        )
    }

    #[test]
    fn is_raised_when_the_object_is_resolved_and_keeps_the_other_keys() {
        let doc = open(&stray_tokens()).unwrap();
        let page = doc.resolve(ObjRef::new(3, 0)).unwrap();
        assert_eq!(
            (page.as_dict().and_then(|d| d.get_int("Rotate")), doc.warnings()),
            (Some(180), vec![Warning::DictionaryKeySkipped { object: Some(3) }])
        );
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        let doc = open_with(&stray_tokens(), strict()).unwrap();
        assert_eq!(
            strict_code(doc.resolve(ObjRef::new(3, 0))),
            Some("dictionary-key-skipped")
        );
    }

    #[test]
    fn is_raised_for_a_trailer_without_an_object() {
        let objects: [&[u8]; 2] = [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [] /Count 0 >>",
        ];
        let doc = open(&classic(&objects, "<< /Size 3 ID /Root 1 0 R >>")).unwrap();
        assert!(doc.warnings().contains(&Warning::DictionaryKeySkipped { object: None }));
    }

    #[test]
    fn an_intact_older_definition_wins_in_repair() {
        // The newer catalog parses only by skipping `0 ]`; repair keeps the older one.
        let mut bytes = one_page();
        let end = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
        bytes.truncate(end);
        bytes.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 ] >>\nendobj\n");
        let doc = open(&bytes).unwrap();
        assert_eq!(
            (
                doc.was_repaired(),
                doc.resolve(ObjRef::new(1, 0))
                    .unwrap()
                    .as_dict()
                    .map(|d| d.get("Pages").cloned())
            ),
            (true, Some(Some(laminapdf_cos::Object::Ref(ObjRef::new(2, 0)))))
        );
    }
}

mod stale_generation {
    use super::*;

    #[test]
    fn is_raised_when_a_reference_generation_differs() {
        let doc = open(&one_page()).unwrap();
        doc.resolve(ObjRef::new(3, 2)).unwrap();
        assert_eq!(
            doc.warnings(),
            [Warning::StaleGeneration {
                object: 3,
                generation: 2
            }]
        );
    }

    #[test]
    fn is_not_raised_for_a_missing_object() {
        let doc = open(&one_page()).unwrap();
        doc.resolve(ObjRef::new(99, 0)).unwrap();
        assert_eq!(doc.warnings(), []);
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        let doc = open_with(&one_page(), strict()).unwrap();
        assert_eq!(strict_code(doc.resolve(ObjRef::new(3, 2))), Some("stale-generation"));
    }
}

mod page_tree_node_skipped {
    use super::*;

    fn missing_kid() -> Vec<u8> {
        file(
            b"<< /Type /Pages /Kids [3 0 R 9 0 R] /Count 1 >>",
            &[b"<< /Type /Page /Parent 2 0 R >>"],
        )
    }

    #[test]
    fn is_raised_with_the_skipped_kid() {
        let doc = open(&missing_kid()).unwrap();
        assert_eq!(
            (doc.page_count(), doc.warnings()),
            (Ok(1), vec![Warning::PageTreeNodeSkipped { object: Some(9) }])
        );
    }

    #[test]
    fn is_raised_for_kids_that_are_not_an_array() {
        let doc = open(&file(b"<< /Type /Pages /Kids 5 /Count 0 >>", &[])).unwrap();
        assert_eq!(
            (doc.page_count(), doc.warnings()),
            (Ok(0), vec![Warning::PageTreeNodeSkipped { object: Some(2) }])
        );
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        let doc = open_with(&missing_kid(), strict()).unwrap();
        assert_eq!(strict_code(doc.pages()), Some("page-tree-node-skipped"));
    }
}

mod page_count_mismatch {
    use super::*;

    fn overstated_count() -> Vec<u8> {
        file(
            b"<< /Type /Pages /Kids [3 0 R] /Count 5 >>",
            &[b"<< /Type /Page /Parent 2 0 R >>"],
        )
    }

    #[test]
    fn is_raised_when_the_root_count_differs_from_the_walk() {
        let doc = open(&overstated_count()).unwrap();
        assert_eq!(
            (doc.page_count(), doc.warnings()),
            (
                Ok(1),
                vec![Warning::PageCountMismatch {
                    object: Some(2),
                    declared: 5,
                    walked: 1
                }]
            )
        );
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        let doc = open_with(&overstated_count(), strict()).unwrap();
        assert_eq!(strict_code(doc.page_count()), Some("page-count-mismatch"));
    }
}

mod flate_truncated {
    use super::*;

    fn truncated_content() -> Vec<u8> {
        let full = flate_encode(&[b' '; 4000]);
        let content = stream_body("/Filter /FlateDecode", full.get(..full.len() - 6).unwrap());
        file(
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            &[b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>", &content],
        )
    }

    fn first_page_content(doc: &Document) -> laminapdf_cos::Result<Vec<u8>> {
        let page = doc.pages()?.remove(0);
        doc.page_content(&page)
    }

    #[test]
    fn is_raised_with_the_stream_object_when_partial_output_is_kept() {
        let doc = open(&truncated_content()).unwrap();
        assert!(first_page_content(&doc).is_ok_and(|c| !c.is_empty()));
        assert_eq!(doc.warnings(), [Warning::FlateTruncated { object: Some(4) }]);
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        let doc = open_with(&truncated_content(), strict()).unwrap();
        assert_eq!(strict_code(first_page_content(&doc)), Some("flate-truncated"));
    }
}

mod objstm_index_mismatch {
    use super::*;

    /// The catalog (object 1) lives in object stream 2, but the xref stream (object 3) gives
    /// it index 7 instead of 0.
    fn stale_index() -> Vec<u8> {
        let mut out = b"%PDF-1.5\n".to_vec();
        let objstm_offset = out.len();
        let members = b"1 0 << /Type /Catalog /Pages << /Type /Pages /Kids [] /Count 0 >> >>";
        out.extend_from_slice(b"2 0 obj ");
        out.extend_from_slice(&stream_body("/Type /ObjStm /N 1 /First 4", members));
        out.extend_from_slice(b" endobj\n");
        let xref_offset = out.len();
        let mut rows = Vec::new();
        for (kind, field2, field3) in [
            (0u8, 0u32, 0xFFFFu16),
            (2, 2, 7),
            (1, u32::try_from(objstm_offset).unwrap(), 0),
            (1, u32::try_from(xref_offset).unwrap(), 0),
        ] {
            rows.push(kind);
            rows.extend_from_slice(&field2.to_be_bytes());
            rows.extend_from_slice(&field3.to_be_bytes());
        }
        out.extend_from_slice(b"3 0 obj ");
        out.extend_from_slice(&stream_body("/Type /XRef /W [1 4 2] /Size 4 /Root 1 0 R", &rows));
        out.extend_from_slice(format!(" endobj\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());
        out
    }

    #[test]
    fn is_raised_when_a_search_finds_the_object() {
        let doc = open(&stale_index()).unwrap();
        assert_eq!(
            (doc.was_repaired(), doc.warnings()),
            (false, vec![Warning::ObjStmIndexMismatch { object: 1 }])
        );
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        assert_eq!(
            strict_code(open_with(&stale_index(), strict())),
            Some("objstm-index-mismatch")
        );
    }
}

#[test]
fn intact_files_open_and_walk_in_strict_mode() {
    for name in ["hello.pdf", "hello-xrefstm.pdf", "imaging-demo.pdf", "text-example.pdf"] {
        let doc = open_with(&fixture(&format!("corpus/{name}")), strict()).unwrap();
        let pages = doc.pages().unwrap();
        for page in &pages {
            doc.page_content(page).unwrap();
        }
        assert_eq!(doc.warnings(), [], "{name}");
    }
}

#[test]
fn every_corpus_file_opens_or_fails_with_a_coded_error_in_strict_mode() {
    for (name, bytes) in corpus() {
        if let Err(e) = open_with(&bytes, strict()) {
            assert!(Error::CODES.contains(&e.code()), "{name}: {e}");
        }
    }
}

#[test]
fn warning_codes_are_kebab_case() {
    assert!(
        Warning::CODES
            .iter()
            .all(|c| c.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'))
    );
}

mod filter_characters_skipped {
    use super::*;
    use std::fmt::Write as _;

    /// A one-page file whose xref stream (object 3) is ASCIIHex-encoded, with `G`, which is
    /// not a hex digit, in its data.
    fn hex_xref_with_a_stray_character() -> Vec<u8> {
        let mut out = b"%PDF-1.5\n".to_vec();
        let catalog = out.len();
        out.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        let pages = out.len();
        out.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [] /Count 0 >> endobj\n");
        let xref = out.len();
        let mut hex = String::new();
        for (kind, offset) in [(0u8, 0usize), (1, catalog), (1, pages), (1, xref)] {
            let offset = u32::try_from(offset).unwrap();
            write!(hex, "{kind:02X}{offset:08X}00 ").unwrap();
        }
        hex.insert(1, 'G');
        hex.push('>');
        out.extend_from_slice(b"3 0 obj ");
        out.extend_from_slice(&stream_body(
            "/Type /XRef /W [1 4 1] /Size 4 /Root 1 0 R /Filter /ASCIIHexDecode",
            hex.as_bytes(),
        ));
        out.extend_from_slice(format!(" endobj\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        out
    }

    /// A page whose content stream (object 4) is ASCIIHex-encoded `BT ET`, with `G` and `!`,
    /// which aren't hex digits, among the digits.
    fn hex_content_with_stray_characters() -> Vec<u8> {
        let content = stream_body("/Filter /ASCIIHexDecode", b"4254 G20!4554>");
        file(
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            &[b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>", &content],
        )
    }

    fn first_page_content(doc: &Document) -> laminapdf_cos::Result<Vec<u8>> {
        let page = doc.pages()?.remove(0);
        doc.page_content(&page)
    }

    #[test]
    fn is_raised_with_the_stream_object_and_the_rest_decodes() {
        let doc = open(&hex_content_with_stray_characters()).unwrap();
        assert_eq!(
            (first_page_content(&doc), doc.warnings()),
            (
                Ok(b"BT ET".to_vec()),
                vec![Warning::FilterCharactersSkipped { object: Some(4) }]
            )
        );
    }

    #[test]
    fn is_an_error_in_strict_mode_for_a_content_stream() {
        let doc = open_with(&hex_content_with_stray_characters(), strict()).unwrap();
        assert_eq!(strict_code(first_page_content(&doc)), Some("filter-characters-skipped"));
    }

    #[test]
    fn is_raised_for_an_xref_stream_and_the_xref_still_reads() {
        let doc = open(&hex_xref_with_a_stray_character()).unwrap();
        assert_eq!(
            (doc.was_repaired(), doc.warnings()),
            (false, vec![Warning::FilterCharactersSkipped { object: None }])
        );
    }

    #[test]
    fn is_an_error_in_strict_mode() {
        assert_eq!(
            strict_code(open_with(&hex_xref_with_a_stray_character(), strict())),
            Some("filter-characters-skipped")
        );
    }
}
