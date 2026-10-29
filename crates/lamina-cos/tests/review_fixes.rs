//! Regressions from the review of PR #13: every fallback in the reader either degrades with
//! the right warning or passes the error on (ADR 0003 §1–3); object-stream members obey the
//! same limits as in-file objects; sources are read safely and shared. (The writer cases, `/Size`
//! never shrinking, moved to `lamina-save`'s `tests/review_fixes.rs` in B3b.)
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use std::sync::Arc;

use common::{classic, open, open_with, stream_body};
use lamina_cos::{
    ByteSource, Document, Error, Limit, Limits, ObjRef, OpenOptions, Position, Source, SourceError, Warning,
};

fn strict() -> OpenOptions {
    OpenOptions::default().with_strict(true)
}

fn with(limits: Limits) -> OpenOptions {
    OpenOptions::default().with_limits(limits)
}

/// Catalog 1, pages 2 with kid 3 (`page`), then `extra` objects from 4 on.
fn one_page_file(page: &[u8], extra: &[&[u8]]) -> Vec<u8> {
    let mut objects: Vec<&[u8]> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page,
    ];
    objects.extend_from_slice(extra);
    classic(&objects, &format!("<< /Size {} /Root 1 0 R >>", objects.len() + 1))
}

/// `extra` objects 4.. as a reference chain 4 → 5 → … ending in `last`.
fn chain(len: u32, last: &str) -> Vec<Vec<u8>> {
    let mut objects: Vec<Vec<u8>> = (4..3 + len).map(|n| format!("{} 0 R", n + 1).into_bytes()).collect();
    objects.push(last.as_bytes().to_vec());
    objects
}

fn refs(objects: &[Vec<u8>]) -> Vec<&[u8]> {
    objects.iter().map(Vec::as_slice).collect()
}

fn strict_warning<T: std::fmt::Debug>(result: lamina_cos::Result<T>) -> Warning {
    match result {
        Err(Error::Strict { warning }) => warning,
        other => panic!("expected Error::Strict, got {other:?}"),
    }
}

mod object_stream_members {
    use super::*;

    /// No xref (so it is repaired): catalog 1 and pages 2 in the file, and object 3 a member
    /// of object stream 4 holding a 9-byte string.
    fn long_string_member() -> Vec<u8> {
        let members = b"3 0 (abcdefghi)";
        let mut out = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n".to_vec();
        out.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [] /Count 0 >> endobj\n4 0 obj ");
        out.extend_from_slice(&stream_body("/Type /ObjStm /N 1 /First 4", members));
        out.extend_from_slice(b" endobj\ntrailer << /Root 1 0 R >>\n");
        out
    }

    #[test]
    fn a_member_over_a_hard_limit_fails_resolve_like_an_in_file_object() {
        let doc = open_with(&long_string_member(), with(Limits::default().with_max_string_length(8))).unwrap();
        assert_eq!(
            doc.resolve(ObjRef::new(3, 0)),
            Err(Error::LimitExceeded {
                limit: Limit::StringLength,
                at: Some(Position::InStream { object: 4, offset: 4 })
            })
        );
    }

    #[test]
    fn a_member_within_the_limit_resolves() {
        let doc = open_with(&long_string_member(), with(Limits::default().with_max_string_length(9))).unwrap();
        assert!(
            doc.resolve(ObjRef::new(3, 0))
                .is_ok_and(|o| matches!(*o, lamina_cos::Object::String(_)))
        );
    }
}

mod page_attributes {
    use super::*;

    #[test]
    fn a_stale_inherited_media_box_is_a_strict_error() {
        let bytes = one_page_file(b"<< /Type /Page /MediaBox 4 1 R >>", &[b"[0 0 10 10]"]);
        let doc = open_with(&bytes, strict()).unwrap();
        assert_eq!(
            strict_warning(doc.pages()),
            Warning::StaleGeneration {
                object: 4,
                generation: 1
            }
        );
    }

    #[test]
    fn a_stale_media_box_element_is_a_strict_error() {
        let bytes = one_page_file(b"<< /Type /Page /MediaBox [0 0 4 1 R 10] >>", &[b"10"]);
        let doc = open_with(&bytes, strict()).unwrap();
        assert_eq!(strict_warning(doc.pages()).code(), "stale-generation");
    }

    #[test]
    fn a_stale_resources_reference_is_a_strict_error() {
        let bytes = one_page_file(b"<< /Type /Page /Resources 4 1 R >>", &[b"<< >>"]);
        let doc = open_with(&bytes, strict()).unwrap();
        assert_eq!(strict_warning(doc.pages()).code(), "stale-generation");
    }

    #[test]
    fn a_stale_rotate_reference_is_a_strict_error() {
        let bytes = one_page_file(b"<< /Type /Page /Rotate 4 1 R >>", &[b"90"]);
        let doc = open_with(&bytes, strict()).unwrap();
        assert_eq!(strict_warning(doc.pages()).code(), "stale-generation");
    }

    #[test]
    fn resources_behind_a_chain_over_the_resolve_depth_fail_the_call() {
        let objects = chain(4, "<< >>");
        let bytes = one_page_file(b"<< /Type /Page /Resources 4 0 R >>", &refs(&objects));
        let doc = open_with(&bytes, with(Limits::default().with_max_resolve_depth(3))).unwrap();
        assert!(matches!(
            doc.pages(),
            Err(Error::LimitExceeded {
                limit: Limit::ResolveDepth,
                ..
            })
        ));
    }

    #[test]
    fn a_media_box_behind_a_chain_over_the_resolve_depth_fails_the_call() {
        let objects = chain(4, "[0 0 10 10]");
        let bytes = one_page_file(b"<< /Type /Page /MediaBox 4 0 R >>", &refs(&objects));
        let doc = open_with(&bytes, with(Limits::default().with_max_resolve_depth(3))).unwrap();
        assert!(matches!(
            doc.pages(),
            Err(Error::LimitExceeded {
                limit: Limit::ResolveDepth,
                ..
            })
        ));
    }

    #[test]
    fn a_malformed_media_box_object_still_reads_as_absent() {
        let bytes = one_page_file(b"<< /Type /Page /MediaBox 4 0 R >>", &[b"[0 0 ]]"]);
        let pages = open(&bytes).unwrap().pages().unwrap();
        assert_eq!(pages[0].media_box, None);
    }

    #[test]
    fn a_stale_page_count_is_a_strict_error() {
        let bytes = classic(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>",
                b"<< /Type /Pages /Kids [3 0 R] /Count 4 1 R >>",
                b"<< /Type /Page >>",
                b"1",
            ],
            "<< /Size 5 /Root 1 0 R >>",
        );
        let doc = open_with(&bytes, strict()).unwrap();
        assert_eq!(strict_warning(doc.pages()).code(), "stale-generation");
    }
}

mod indirect_length {
    use super::*;

    fn stale_length() -> Vec<u8> {
        one_page_file(b"<< /Length 4 1 R >>\nstream\nabc\nendstream", &[b"3"])
    }

    #[test]
    fn a_stale_length_reference_reports_stale_generation_in_strict_mode() {
        let doc = open_with(&stale_length(), strict()).unwrap();
        assert_eq!(
            strict_warning(doc.resolve(ObjRef::new(3, 0))),
            Warning::StaleGeneration {
                object: 4,
                generation: 1
            }
        );
    }

    #[test]
    fn a_length_over_the_resolve_depth_degrades_with_stream_length_recovered() {
        let objects = chain(4, "3");
        let bytes = one_page_file(b"<< /Length 4 0 R >>\nstream\nabc\nendstream", &refs(&objects));
        let doc = open_with(&bytes, with(Limits::default().with_max_resolve_depth(2))).unwrap();
        doc.resolve(ObjRef::new(3, 0)).unwrap();
        assert_eq!(doc.warnings(), [Warning::StreamLengthRecovered { object: 3 }]);
    }
}

/// A source without `as_slice`, counting how often it is read.
struct Counted {
    data: Vec<u8>,
    reads: std::sync::atomic::AtomicUsize,
}

impl ByteSource for Counted {
    fn len(&self) -> u64 {
        u64::try_from(self.data.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tail = self.data.get(usize::try_from(offset).unwrap()..).unwrap_or_default();
        let n = tail.len().min(buf.len());
        buf[..n].copy_from_slice(&tail[..n]);
        Ok(n)
    }
}

/// Claims a terabyte, holds a few bytes.
struct Liar;

impl ByteSource for Liar {
    fn len(&self) -> u64 {
        1 << 40
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        let data: &[u8] = b"%PDF-1.7\n";
        let tail = data.get(usize::try_from(offset).unwrap()..).unwrap_or_default();
        let n = tail.len().min(buf.len());
        buf[..n].copy_from_slice(&tail[..n]);
        Ok(n)
    }
}

mod sources {
    use super::*;

    #[test]
    fn a_source_that_overstates_its_length_is_an_error_not_an_allocation() {
        // Windowed reading (B2d): the first 64 KiB window ends after 9 bytes.
        assert_eq!(
            Document::open(Liar, OpenOptions::default()).err(),
            Some(Error::Source(SourceError::Failed {
                offset: 9,
                len: 64 * 1024 - 9
            }))
        );
    }

    #[test]
    fn documents_opened_from_one_source_share_its_window_cache() {
        let counted = Arc::new(Counted {
            data: common::fixture("corpus/hello.pdf"),
            reads: 0.into(),
        });
        let source = Source::from(Arc::clone(&counted) as Arc<dyn ByteSource>);
        let a = Document::open(source.clone(), OpenOptions::default()).unwrap();
        let b = Document::open(source, OpenOptions::default()).unwrap();
        a.pages().unwrap();
        b.pages().unwrap();
        assert_eq!(counted.reads.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn a_shared_source_is_read_once() {
        let counted = Arc::new(Counted {
            data: common::fixture("corpus/hello.pdf"),
            reads: 0.into(),
        });
        let source = Source::from(Arc::clone(&counted) as Arc<dyn ByteSource>);
        for _ in 0..3 {
            Document::open(source.clone(), OpenOptions::default()).unwrap();
        }
        assert_eq!(counted.reads.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
