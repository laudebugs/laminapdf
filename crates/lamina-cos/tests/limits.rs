//! ADR 0003 §3–4: each hard limit succeeds at the limit and fails one past it with
//! `LimitExceeded`, and a tolerant path that hits it degrades with its usual warning; each soft
//! limit drops the item with its warning.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use common::{classic, open_with, stream_body};
use lamina_cos::{Error, Limit, Limits, ObjRef, OpenOptions, Warning};
use lamina_syntax::filters::flate_encode;

fn with(limits: Limits) -> OpenOptions {
    OpenOptions::default().with_limits(limits)
}

fn exceeded(limit: Limit) -> impl Fn(&Error) -> bool {
    move |e| matches!(e, Error::LimitExceeded { limit: l, .. } if *l == limit)
}

fn codes(warnings: &[Warning]) -> Vec<&'static str> {
    warnings.iter().map(Warning::code).collect()
}

/// Catalog 1, pages 2 with one kid (3), plus `extra` objects from 3 on.
fn one_page_file(page: &[u8], extra: &[&[u8]]) -> Vec<u8> {
    let mut objects: Vec<&[u8]> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page,
    ];
    objects.extend_from_slice(extra);
    classic(&objects, &format!("<< /Size {} /Root 1 0 R >>", objects.len() + 1))
}

/// A one-page file whose page has `content` as its `/Contents` (object 4).
fn content_file(content: &[u8]) -> Vec<u8> {
    one_page_file(b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>", &[content])
}

fn first_page_content(bytes: &[u8], limits: Limits) -> lamina_cos::Result<Vec<u8>> {
    let doc = open_with(bytes, with(limits)).unwrap();
    let page = doc.pages().unwrap().remove(0);
    doc.page_content(&page)
}

/// A file without any xref (so it is repaired) whose only page, object 3, lives in object
/// stream 4; the object stream's body is `objstm` (a stream body with `/Type /ObjStm`).
fn page_in_object_stream(objstm: &[u8]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n".to_vec();
    out.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n4 0 obj ");
    out.extend_from_slice(objstm);
    out.extend_from_slice(b" endobj\ntrailer << /Root 1 0 R >>\n");
    out
}

/// The object-stream payload holding page 3, padded to `len` decoded bytes.
fn page_members(len: usize) -> Vec<u8> {
    let mut members = b"3 0 << /Type /Page /Parent 2 0 R >>".to_vec();
    members.resize(len, b' ');
    members
}

/// `(page_count, warning codes)` after opening `bytes` with `limits`.
fn walk(bytes: &[u8], limits: Limits) -> (usize, Vec<&'static str>) {
    let doc = open_with(bytes, with(limits)).unwrap();
    let count = doc.page_count().unwrap();
    (count, codes(&doc.warnings()))
}

mod nesting_depth {
    use super::*;

    fn nested(depth: usize) -> Vec<u8> {
        [vec![b'['; depth], vec![b']'; depth]].concat()
    }

    #[test]
    fn resolve_accepts_nesting_at_the_limit() {
        let bytes = one_page_file(b"<< /Type /Page >>", &[&nested(5)]);
        let doc = open_with(&bytes, with(Limits::default().with_max_nesting_depth(5))).unwrap();
        assert!(doc.resolve(ObjRef::new(4, 0)).is_ok());
    }

    #[test]
    fn resolve_rejects_nesting_past_the_limit() {
        let bytes = one_page_file(b"<< /Type /Page >>", &[&nested(6)]);
        let doc = open_with(&bytes, with(Limits::default().with_max_nesting_depth(5))).unwrap();
        assert!(
            doc.resolve(ObjRef::new(4, 0))
                .is_err_and(|e| exceeded(Limit::NestingDepth)(&e))
        );
    }

    #[test]
    fn page_walk_skips_a_kid_past_the_limit_with_a_warning() {
        let page = [b"<< /Type /Page /X ".as_slice(), &nested(6), b" >>"].concat();
        let bytes = one_page_file(&page, &[]);
        // The page dictionary itself is one level, so the array adds 6 more.
        assert_eq!(
            walk(&bytes, Limits::default().with_max_nesting_depth(6)),
            (0, vec!["page-tree-node-skipped", "page-count-mismatch"])
        );
    }
}

mod string_length {
    use super::*;

    #[test]
    fn resolve_accepts_a_string_at_the_limit() {
        let bytes = one_page_file(b"<< /Type /Page >>", &[b"(abcdefgh)"]);
        let doc = open_with(&bytes, with(Limits::default().with_max_string_length(8))).unwrap();
        assert!(doc.resolve(ObjRef::new(4, 0)).is_ok());
    }

    #[test]
    fn resolve_rejects_a_string_past_the_limit() {
        let bytes = one_page_file(b"<< /Type /Page >>", &[b"(abcdefghi)"]);
        let doc = open_with(&bytes, with(Limits::default().with_max_string_length(8))).unwrap();
        assert!(
            doc.resolve(ObjRef::new(4, 0))
                .is_err_and(|e| exceeded(Limit::StringLength)(&e))
        );
    }

    #[test]
    fn page_walk_skips_a_kid_with_a_string_past_the_limit() {
        let bytes = one_page_file(b"<< /Type /Page /T (abcdefghi) >>", &[]);
        assert_eq!(
            walk(&bytes, Limits::default().with_max_string_length(8)),
            (0, vec!["page-tree-node-skipped", "page-count-mismatch"])
        );
    }
}

mod stream_decoded {
    use super::*;

    fn flate_content(len: usize) -> Vec<u8> {
        stream_body("/Filter /FlateDecode", &flate_encode(&vec![b' '; len]))
    }

    #[test]
    fn page_content_at_the_limit_decodes() {
        let bytes = content_file(&flate_content(1000));
        let limits = Limits::default().with_max_stream_decoded(1000);
        assert_eq!(first_page_content(&bytes, limits).map(|c| c.len()), Ok(1000));
    }

    #[test]
    fn page_content_past_the_limit_is_refused() {
        let bytes = content_file(&flate_content(1001));
        let limits = Limits::default().with_max_stream_decoded(1000);
        assert_eq!(
            first_page_content(&bytes, limits),
            Err(Error::LimitExceeded {
                limit: Limit::StreamDecoded,
                at: None
            })
        );
    }

    fn compressed_page_file() -> Vec<u8> {
        let members = page_members(200);
        let objstm = stream_body(
            "/Type /ObjStm /N 1 /First 4 /Filter /FlateDecode",
            &flate_encode(&members),
        );
        page_in_object_stream(&objstm)
    }

    #[test]
    fn repair_reads_an_object_stream_at_the_limit() {
        assert_eq!(
            walk(&compressed_page_file(), Limits::default().with_max_stream_decoded(200)),
            (1, vec!["xref-repaired"])
        );
    }

    #[test]
    fn repair_skips_an_object_stream_past_the_limit_and_the_walk_skips_its_page() {
        assert_eq!(
            walk(&compressed_page_file(), Limits::default().with_max_stream_decoded(199)),
            (
                0,
                vec!["xref-repaired", "page-tree-node-skipped", "page-count-mismatch"]
            )
        );
    }
}

mod decoded_total {
    use super::*;

    /// A page whose `/Contents` are two Flate streams of 600 bytes each (objects 4 and 5).
    fn two_part_file() -> Vec<u8> {
        let part = stream_body("/Filter /FlateDecode", &flate_encode(&[b' '; 600]));
        one_page_file(
            b"<< /Type /Page /Parent 2 0 R /Contents [4 0 R 5 0 R] >>",
            &[&part, &part],
        )
    }

    #[test]
    fn page_content_within_the_total_decodes() {
        let limits = Limits::default().with_max_decoded_total(1200);
        assert_eq!(first_page_content(&two_part_file(), limits).map(|c| c.len()), Ok(1201));
    }

    #[test]
    fn page_content_past_the_total_is_refused() {
        let limits = Limits::default().with_max_decoded_total(1199);
        assert_eq!(
            first_page_content(&two_part_file(), limits),
            Err(Error::LimitExceeded {
                limit: Limit::DecodedTotal,
                at: None
            })
        );
    }

    #[test]
    fn the_total_is_per_call_not_per_document() {
        let doc = open_with(&two_part_file(), with(Limits::default().with_max_decoded_total(1200))).unwrap();
        let page = doc.pages().unwrap().remove(0);
        let lens: Vec<_> = (0..3).map(|_| doc.page_content(&page).map(|c| c.len())).collect();
        assert_eq!(lens, [Ok(1201), Ok(1201), Ok(1201)]);
    }

    #[test]
    fn repair_skips_an_object_stream_past_the_total() {
        let members = page_members(200);
        let objstm = stream_body(
            "/Type /ObjStm /N 1 /First 4 /Filter /FlateDecode",
            &flate_encode(&members),
        );
        assert_eq!(
            walk(
                &page_in_object_stream(&objstm),
                Limits::default().with_max_decoded_total(199)
            ),
            (
                0,
                vec!["xref-repaired", "page-tree-node-skipped", "page-count-mismatch"]
            )
        );
    }
}

mod filter_chain {
    use super::*;

    fn chained(data: &[u8], n: usize) -> Vec<u8> {
        let encoded = (0..n).fold(data.to_vec(), |acc, _| flate_encode(&acc));
        let filters = vec!["/FlateDecode"; n].join(" ");
        stream_body(&format!("/Filter [{filters}]"), &encoded)
    }

    #[test]
    fn page_content_with_a_chain_at_the_limit_decodes() {
        let bytes = content_file(&chained(b"q Q", 3));
        let limits = Limits::default().with_max_filter_chain(3);
        assert_eq!(first_page_content(&bytes, limits), Ok(b"q Q".to_vec()));
    }

    #[test]
    fn page_content_with_a_chain_past_the_limit_is_refused() {
        let bytes = content_file(&chained(b"q Q", 4));
        let limits = Limits::default().with_max_filter_chain(3);
        assert_eq!(
            first_page_content(&bytes, limits),
            Err(Error::LimitExceeded {
                limit: Limit::FilterChain,
                at: None
            })
        );
    }

    #[test]
    fn repair_skips_an_object_stream_whose_chain_is_past_the_limit() {
        let members = page_members(40);
        let encoded = flate_encode(&flate_encode(&members));
        let objstm = stream_body(
            "/Type /ObjStm /N 1 /First 4 /Filter [/FlateDecode /FlateDecode]",
            &encoded,
        );
        assert_eq!(
            walk(
                &page_in_object_stream(&objstm),
                Limits::default().with_max_filter_chain(1)
            ),
            (
                0,
                vec!["xref-repaired", "page-tree-node-skipped", "page-count-mismatch"]
            )
        );
    }
}

mod resolve_depth {
    use super::*;

    /// Objects 4.. form a reference chain of `len` objects ending in an integer.
    fn chain_file(len: u32) -> Vec<u8> {
        let mut extra: Vec<Vec<u8>> = (4..3 + len).map(|n| format!("{} 0 R", n + 1).into_bytes()).collect();
        extra.push(b"42".to_vec());
        let refs: Vec<&[u8]> = extra.iter().map(Vec::as_slice).collect();
        one_page_file(b"<< /Type /Page >>", &refs)
    }

    #[test]
    fn resolve_follows_a_chain_at_the_limit() {
        let doc = open_with(&chain_file(5), with(Limits::default().with_max_resolve_depth(5))).unwrap();
        assert_eq!(doc.resolve(ObjRef::new(4, 0)).unwrap().as_int(), Some(42));
    }

    #[test]
    fn resolve_rejects_a_chain_past_the_limit() {
        let doc = open_with(&chain_file(6), with(Limits::default().with_max_resolve_depth(5))).unwrap();
        assert_eq!(
            doc.resolve(ObjRef::new(4, 0)),
            Err(Error::LimitExceeded {
                limit: Limit::ResolveDepth,
                at: Some(9)
            })
        );
    }

    #[test]
    fn page_walk_skips_a_kid_behind_a_chain_past_the_limit() {
        // Kid 3 → 4 → 5 → page dictionary 6: a chain of four objects.
        let bytes = classic(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>",
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                b"4 0 R",
                b"5 0 R",
                b"6 0 R",
                b"<< /Type /Page >>",
            ],
            "<< /Size 7 /Root 1 0 R >>",
        );
        assert_eq!(
            walk(&bytes, Limits::default().with_max_resolve_depth(3)),
            (0, vec!["page-tree-node-skipped", "page-count-mismatch"])
        );
    }
}

mod page_tree_depth {
    use super::*;

    /// A page tree with `levels` `/Pages` nodes below the root and one page at the bottom.
    fn deep_tree(levels: u32) -> Vec<u8> {
        let mut bodies = vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()];
        for n in 2..=levels + 2 {
            bodies.push(format!("<< /Type /Pages /Kids [{} 0 R] /Count 1 >>", n + 1).into_bytes());
        }
        bodies.push(b"<< /Type /Page >>".to_vec());
        let refs: Vec<&[u8]> = bodies.iter().map(Vec::as_slice).collect();
        classic(&refs, &format!("<< /Size {} /Root 1 0 R >>", refs.len() + 1))
    }

    #[test]
    fn page_walk_reaches_a_page_at_the_limit() {
        // Root, 4 intermediate levels, then the page: 5 `/Kids` hops.
        assert_eq!(
            walk(&deep_tree(4), Limits::default().with_max_page_tree_depth(5)),
            (1, vec![])
        );
    }

    #[test]
    fn page_walk_skips_a_subtree_past_the_limit_with_a_warning() {
        assert_eq!(
            walk(&deep_tree(5), Limits::default().with_max_page_tree_depth(5)),
            (0, vec!["page-tree-node-skipped", "page-count-mismatch"])
        );
    }
}

mod xref_sections {
    use super::*;

    fn two_sections() -> Vec<u8> {
        common::fixture("corpus/hello-incremental.pdf")
    }

    #[test]
    fn a_chain_at_the_limit_is_used() {
        let doc = open_with(&two_sections(), with(Limits::default().with_max_xref_sections(2))).unwrap();
        assert_eq!((doc.was_repaired(), doc.revisions()), (false, 2));
    }

    #[test]
    fn a_chain_past_the_limit_is_abandoned_for_repair_with_a_warning() {
        let doc = open_with(&two_sections(), with(Limits::default().with_max_xref_sections(1))).unwrap();
        assert_eq!(
            (doc.was_repaired(), codes(&doc.warnings())),
            (true, vec!["xref-repaired"])
        );
    }
}

mod object_number {
    use super::*;

    #[test]
    fn an_entry_at_the_limit_is_kept() {
        let bytes = one_page_file(b"<< /Type /Page >>", &[b"42"]);
        let doc = open_with(&bytes, with(Limits::default().with_max_object_number(4))).unwrap();
        assert_eq!((doc.object_numbers().max(), doc.warnings()), (Some(4), vec![]));
    }

    #[test]
    fn an_entry_past_the_limit_is_dropped_with_a_warning() {
        let bytes = one_page_file(b"<< /Type /Page >>", &[b"42"]);
        let doc = open_with(&bytes, with(Limits::default().with_max_object_number(3))).unwrap();
        assert_eq!(
            (doc.object_numbers().max(), doc.warnings()),
            (Some(3), vec![Warning::ObjectNumberOutOfRange { object: 4 }])
        );
    }

    #[test]
    fn a_dropped_entry_resolves_to_null() {
        let bytes = one_page_file(b"<< /Type /Page >>", &[b"42"]);
        let doc = open_with(&bytes, with(Limits::default().with_max_object_number(3))).unwrap();
        assert_eq!(doc.resolve(ObjRef::new(4, 0)).unwrap().as_int(), None);
    }
}

mod warnings_kept {
    use super::*;

    /// Five stale references (generation 1 against entries of generation 0), each a warning.
    fn stale_references() -> (Vec<u8>, Vec<ObjRef>) {
        let bytes = one_page_file(b"<< /Type /Page >>", &[b"1", b"2", b"3", b"4", b"5"]);
        (bytes, (4..9).map(|n| ObjRef::new(n, 1)).collect())
    }

    #[test]
    fn warnings_at_the_limit_are_all_kept() {
        let (bytes, refs) = stale_references();
        let doc = open_with(&bytes, with(Limits::default().with_max_warnings(5))).unwrap();
        for r in &refs {
            drop(doc.resolve(*r));
        }
        assert_eq!(codes(&doc.warnings()), vec!["stale-generation"; 5]);
    }

    #[test]
    fn warnings_past_the_limit_are_dropped_after_one_truncation_warning() {
        let (bytes, refs) = stale_references();
        let doc = open_with(&bytes, with(Limits::default().with_max_warnings(3))).unwrap();
        for r in &refs {
            drop(doc.resolve(*r));
        }
        assert_eq!(
            codes(&doc.warnings()),
            [
                "stale-generation",
                "stale-generation",
                "stale-generation",
                "warnings-truncated"
            ]
        );
    }
}

#[test]
fn unlimited_limits_open_and_read_the_corpus() {
    for (name, bytes) in common::corpus() {
        let doc = open_with(&bytes, with(Limits::unlimited())).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(doc.pages().is_ok(), "{name}");
    }
}
