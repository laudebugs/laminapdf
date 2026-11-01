//! Which errors end extraction (todo Q2e, ADR 0003): a source that fails while a font loads
//! fails the page, and a font object that is merely malformed reads as absent.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use laminapdf_cos::{ByteSource, Error, OpenOptions, SourceError};
use laminapdf_model::Document;
use laminapdf_save::Writer;
use laminapdf_syntax::{Dict, Object, Stream};
use laminapdf_text::TextExtractor;

/// Bytes that fail to read in `failing` once `armed`, as a file whose disk starts failing in
/// one place after the document opened would.
struct FailingRange {
    data: Vec<u8>,
    failing: Range<u64>,
    armed: AtomicBool,
}

impl FailingRange {
    fn new(data: Vec<u8>, failing: Range<u64>) -> Self {
        Self {
            data,
            failing,
            armed: AtomicBool::new(false),
        }
    }
}

impl ByteSource for FailingRange {
    fn len(&self) -> u64 {
        u64::try_from(self.data.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        let end = offset.saturating_add(u64::try_from(buf.len()).unwrap());
        if self.armed.load(Ordering::Relaxed) && offset < self.failing.end && self.failing.start < end {
            return Err(SourceError::Failed {
                offset,
                len: u64::try_from(buf.len()).unwrap(),
            });
        }
        let tail = self.data.get(usize::try_from(offset).unwrap()..).unwrap_or_default();
        let n = tail.len().min(buf.len());
        buf[..n].copy_from_slice(&tail[..n]);
        Ok(n)
    }
}

/// A one-page file whose font's `/ToUnicode` object, `to_unicode`, sits between two 256 KiB
/// paddings, so no read of the header, the trailer or the other objects touches it. Returns the bytes and the
/// byte range of the `/ToUnicode` object.
fn file(to_unicode: Object) -> (Vec<u8>, Range<u64>) {
    let dict = |entries: Vec<(&str, Object)>| entries.into_iter().collect::<Dict>();
    let mut w = Writer::new(1, 7);
    let padding = || Stream::new(Dict::new(), vec![b' '; 256 * 1024]);
    w.add(padding());
    let cmap = w.add(to_unicode);
    w.add(padding());
    let font = w.add(dict(vec![
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
        ("ToUnicode", cmap.into()),
    ]));
    let content = w.add(Stream::new(Dict::new(), b"BT /F1 12 Tf 72 700 Td (A) Tj ET".to_vec()));
    let pages = w.reserve();
    let page = w.add(dict(vec![
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ("Contents", content.into()),
        (
            "Resources",
            dict(vec![("Font", dict(vec![("F1", font.into())]).into())]).into(),
        ),
    ]));
    w.set(
        pages,
        dict(vec![
            ("Type", Object::name("Pages")),
            ("Kids", vec![page.into()].into()),
            ("Count", 1.into()),
        ]),
    );
    let root = w.add(dict(vec![("Type", Object::name("Catalog")), ("Pages", pages.into())]));
    let bytes = w.finish(&[("Root", root)].into_iter().collect()).expect("writes");
    let marker = format!("{} 0 obj", cmap.num);
    let start = bytes
        .windows(marker.len())
        .position(|w| w == marker.as_bytes())
        .expect("the /ToUnicode object");
    let end = start + 64;
    (bytes, u64::try_from(start).unwrap()..u64::try_from(end).unwrap())
}

/// The text of the first page, extracted after `source` is armed.
fn extract(source: FailingRange) -> laminapdf_cos::Result<laminapdf_text::PageText> {
    let source = Arc::new(source);
    let doc = Document::open(Arc::clone(&source) as Arc<dyn ByteSource>, OpenOptions::default()).expect("opens");
    let pages = doc.pages().expect("pages");
    let page = pages.get(0).expect("a page");
    source.armed.store(true, Ordering::Relaxed);
    TextExtractor::new().page(&doc, &page)
}

#[test]
fn extract_should_return_the_source_error_when_a_font_object_cannot_be_read() {
    let (data, failing) = file(Stream::new(Dict::new(), b"not read".to_vec()).into());

    let result = extract(FailingRange::new(data, failing));

    assert!(
        matches!(result, Err(Error::Source(SourceError::Failed { .. }))),
        "{result:?}"
    );
}

#[test]
fn extract_should_read_a_malformed_to_unicode_as_absent() {
    let (data, _) = file(Object::Integer(7));

    let text = extract(FailingRange::new(data, 0..0)).expect("extracts");

    assert_eq!(text.lines(), ["A"]);
}
