//! ADR 0026 §1.1: every position an error carries is tagged with what it counts in. The syntax
//! layer only sees the buffer it parsed; the reader turns that into a file offset (for the
//! whole file, or a window of it) or an offset in an object stream's decoded data.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use std::sync::Arc;

use common::NoSlice;
use common::classic::{classic, stream_body};
use laminapdf_cos::{ByteSource, Document, Error, ErrorKind, Limit, Limits, Location, ObjRef, OpenOptions, Position};

fn limits() -> OpenOptions {
    OpenOptions::default().with_limits(Limits::default().with_max_nesting_depth(2))
}

/// Catalog 1, a 100 000-byte string 2 (so object 3 is past the first 64 KiB window), and
/// object 3 nested one level deeper than [`limits`] allows. Returns the file and the file
/// offset where the parser stopped: just past the third `[`.
fn deep_object_past_the_first_window() -> (Vec<u8>, u64) {
    let filler = [b"(".as_slice(), &vec![b'a'; 100_000], b")"].concat();
    let bytes = classic(
        &[b"<< /Type /Catalog /Pages 9 0 R >>", &filler, b"[[[1]]]"],
        "<< /Size 4 /Root 1 0 R >>",
    );
    let object = bytes.windows(8).position(|w| w == b"3 0 obj\n").unwrap();
    (bytes, u64::try_from(object + b"3 0 obj\n".len() + 3).unwrap())
}

fn nesting_error(doc: &Document) -> Error {
    doc.resolve(ObjRef::new(3, 0)).unwrap_err()
}

#[test]
fn a_nesting_error_in_the_file_is_at_a_file_offset() {
    let (bytes, at) = deep_object_past_the_first_window();
    let doc = Document::open(bytes, limits()).unwrap();
    assert_eq!(
        nesting_error(&doc),
        Error::LimitExceeded {
            limit: Limit::NestingDepth,
            at: Some(Position::FileOffset(at)),
        }
    );
}

#[test]
fn a_nesting_error_read_through_a_window_is_at_the_same_file_offset() {
    let (bytes, at) = deep_object_past_the_first_window();
    let source: Arc<dyn ByteSource> = Arc::new(NoSlice(bytes));
    let doc = Document::open(source, limits()).unwrap();
    assert_eq!(
        nesting_error(&doc),
        Error::LimitExceeded {
            limit: Limit::NestingDepth,
            at: Some(Position::FileOffset(at)),
        }
    );
}

#[test]
fn a_syntax_error_read_through_a_window_is_at_the_same_file_offset_as_in_memory() {
    let filler = [b"(".as_slice(), &vec![b'a'; 100_000], b")"].concat();
    let bytes = classic(
        &[b"<< /Type /Catalog /Pages 9 0 R >>", &filler, b"[1 >> 2]"],
        "<< /Size 4 /Root 1 0 R >>",
    );
    let in_memory = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
    let windowed = Document::open(Arc::new(NoSlice(bytes)) as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    let errors = [&in_memory, &windowed].map(|d| d.resolve(ObjRef::new(3, 0)).unwrap_err().location());
    assert!(errors[0].offset.is_some_and(|o| o > 100_000), "{errors:?}");
    assert_eq!(errors[0], errors[1]);
}

/// Catalog 1, pages 2, and object 3 a member of object stream 4 nested one level deeper than
/// [`limits`] allows; no xref, so it is repaired. Returns the file and the offset in the
/// stream's decoded data where the parser stopped (just past the third `[`; `/First` is 4).
fn deep_member() -> (Vec<u8>, u64) {
    let members = b"3 0 [[[1]]]";
    let mut out = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n".to_vec();
    out.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [] /Count 0 >> endobj\n4 0 obj ");
    out.extend_from_slice(&stream_body("/Type /ObjStm /N 1 /First 4", members));
    out.extend_from_slice(b" endobj\ntrailer << /Root 1 0 R >>\n");
    (out, 4 + 3)
}

#[test]
fn a_limit_hit_inside_an_object_stream_is_at_an_offset_in_its_data() {
    let (bytes, at) = deep_member();
    let doc = Document::open(bytes, limits()).unwrap();
    assert_eq!(
        doc.resolve(ObjRef::new(3, 0)),
        Err(Error::LimitExceeded {
            limit: Limit::NestingDepth,
            at: Some(Position::InStream { object: 4, offset: at }),
        })
    );
}

#[test]
fn an_offset_in_an_object_stream_is_located_by_object_and_stream_offset() {
    let (bytes, at) = deep_member();
    let doc = Document::open(bytes, limits()).unwrap();
    let location = doc.resolve(ObjRef::new(3, 0)).unwrap_err().location();
    assert_eq!(
        location,
        Location {
            object: Some(4),
            stream_offset: Some(at),
            ..Location::NONE
        }
    );
}

#[test]
fn a_reference_chain_limit_is_at_an_object() {
    // 3 → 4 → 5 → 6: the third hop is past a resolve depth of 2.
    let bytes = classic(
        &[
            b"<< /Type /Catalog /Pages 9 0 R >>",
            b"null",
            b"4 0 R",
            b"5 0 R",
            b"6 0 R",
            b"1",
        ],
        "<< /Size 7 /Root 1 0 R >>",
    );
    let options = OpenOptions::default().with_limits(Limits::default().with_max_resolve_depth(2));
    let doc = Document::open(bytes, options).unwrap();
    let e = doc.resolve(ObjRef::new(3, 0)).unwrap_err();
    assert!(
        matches!(
            e,
            Error::LimitExceeded {
                limit: Limit::ResolveDepth,
                at: Some(Position::Object(_)),
            }
        ),
        "{e:?}"
    );
}

#[test]
fn a_limit_error_is_of_kind_limit() {
    let (bytes, _) = deep_member();
    let doc = Document::open(bytes, limits()).unwrap();
    assert_eq!(doc.resolve(ObjRef::new(3, 0)).unwrap_err().kind(), ErrorKind::Limit);
}

#[test]
fn a_limit_message_names_the_limit_and_the_tagged_position() {
    let e = Error::LimitExceeded {
        limit: Limit::NestingDepth,
        at: Some(Position::InStream { object: 4, offset: 6 }),
    };
    assert_eq!(
        e.to_string(),
        "max_nesting_depth exceeded at byte 6 of object 4's stream data"
    );
}
