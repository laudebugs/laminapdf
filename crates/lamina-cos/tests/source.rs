//! ADR 0005 §2, §7: `Document` owns its input through a `ByteSource`, is `Send`, and several
//! documents can share one source.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use std::sync::Arc;

use common::fixture;
use lamina_cos::{ByteSource, Document, Error, OpenOptions, SourceError};

/// A source that is not in memory (no `as_slice`), served in small reads.
struct Chunked(Vec<u8>);

impl ByteSource for Chunked {
    fn len(&self) -> u64 {
        u64::try_from(self.0.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        let tail = self.0.get(usize::try_from(offset).unwrap()..).unwrap_or_default();
        let n = tail.len().min(buf.len()).min(7);
        buf[..n].copy_from_slice(&tail[..n]);
        Ok(n)
    }
}

/// A source whose bytes have not arrived.
struct NotFetched;

impl ByteSource for NotFetched {
    fn len(&self) -> u64 {
        100
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        Err(SourceError::Pending {
            offset,
            len: u64::try_from(buf.len()).unwrap(),
        })
    }
}

fn page_count(doc: &lamina_cos::Result<Document>) -> Option<usize> {
    doc.as_ref().ok().and_then(|d| d.page_count().ok())
}

fn hello() -> Vec<u8> {
    fixture("corpus/hello.pdf")
}

#[test]
fn opens_a_vec() {
    assert_eq!(page_count(&Document::open(hello(), OpenOptions::default())), Some(1));
}

#[test]
fn opens_an_arc_slice() {
    let shared: Arc<[u8]> = hello().into();
    assert_eq!(page_count(&Document::open(shared, OpenOptions::default())), Some(1));
}

#[test]
fn opens_a_static_slice() {
    let leaked: &'static [u8] = Box::leak(hello().into_boxed_slice());
    assert_eq!(page_count(&Document::open(leaked, OpenOptions::default())), Some(1));
}

#[test]
fn opens_a_source_without_an_in_memory_slice() {
    let doc = Document::open(Chunked(hello()), OpenOptions::default()).unwrap();
    assert_eq!((doc.page_count(), doc.data()), (Ok(1), hello().as_slice()));
}

#[test]
fn reports_bytes_that_are_not_fetched_yet() {
    assert_eq!(
        Document::open(NotFetched, OpenOptions::default()).err(),
        Some(Error::DataPending { ranges: vec![(0, 100)] })
    );
}

#[test]
fn documents_share_one_source_without_copying() {
    let source: Arc<dyn ByteSource> = Arc::new(hello());
    let a = Document::open(Arc::clone(&source), OpenOptions::default()).unwrap();
    let b = Document::open(Arc::clone(&source), OpenOptions::default()).unwrap();
    assert!(Arc::ptr_eq(a.source(), b.source()) && std::ptr::eq(a.data(), b.data()));
}

#[test]
fn in_memory_input_is_parsed_in_place() {
    let source: Arc<dyn ByteSource> = Arc::new(hello());
    let doc = Document::open(Arc::clone(&source), OpenOptions::default()).unwrap();
    assert!(std::ptr::eq(doc.data(), source.as_slice().unwrap()));
}

#[test]
fn a_document_can_move_to_another_thread() {
    let doc = Document::open(hello(), OpenOptions::default()).unwrap();
    let count = std::thread::spawn(move || doc.page_count()).join().unwrap();
    assert_eq!(count, Ok(1));
}

#[test]
fn one_document_per_thread_over_a_shared_source() {
    let source: Arc<dyn ByteSource> = Arc::new(fixture("corpus/imaging-demo.pdf"));
    let counts: Vec<_> = (0..4)
        .map(|_| {
            let source = Arc::clone(&source);
            std::thread::spawn(move || Document::open(source, OpenOptions::default())?.page_count())
        })
        .map(|t| t.join().unwrap())
        .collect();
    assert!(counts.iter().all(|c| *c == Ok(1)));
}
