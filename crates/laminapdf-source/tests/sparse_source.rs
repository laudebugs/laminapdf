//! `SparseSource` and the retry loop: gaps, merges, budget, corpus parity, bytes fetched.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::single_range_in_vec_init,
    clippy::unnecessary_wraps,
    reason = "tests: ranges are the values under test; fetch closures return Result"
)]

mod common;

use std::cell::{Cell, RefCell};
use std::convert::Infallible;
use std::ops::Range;
use std::sync::Arc;

use common::{PAGES, Step, facts, first_difference, in_memory_facts, inputs, multi_page, show};
use laminapdf::{ByteSource, Document, OpenOptions};
use laminapdf_cos::SourceError;
use laminapdf_source::{RetryError, RetryOptions, SparseSource, SupplyError, open_with, retry};

fn read(source: &SparseSource, offset: u64, len: usize) -> Result<Vec<u8>, SourceError> {
    let mut buf = vec![0; len];
    let n = source.read_at(offset, &mut buf)?;
    buf.truncate(n);
    Ok(buf)
}

/// Reads `len` bytes at `offset` the way the reader does: again after each short read.
fn read_all(source: &SparseSource, offset: u64, len: usize) -> Result<Vec<u8>, SourceError> {
    let mut out = Vec::new();
    while out.len() < len {
        let part = read(source, offset + out.len() as u64, len - out.len())?;
        if part.is_empty() {
            break;
        }
        out.extend(part);
    }
    Ok(out)
}

#[test]
fn an_empty_source_answers_pending_for_any_read() {
    let s = SparseSource::new(100);
    assert_eq!(read(&s, 10, 20), Err(SourceError::Pending { offset: 10, len: 20 }));
    assert_eq!(
        read(&s, 90, 50),
        Err(SourceError::Pending { offset: 90, len: 10 }),
        "clamped to the end"
    );
    assert_eq!(read(&s, 100, 5), Ok(vec![]), "past the end reads nothing");
}

#[test]
fn a_supplied_range_reads_back_and_a_gap_after_it_is_pending() {
    let s = SparseSource::new(100);
    s.supply(10, b"abcde").unwrap();
    assert_eq!(read(&s, 11, 3), Ok(b"bcd".to_vec()));
    assert_eq!(read(&s, 12, 10), Ok(b"cde".to_vec()), "a short read up to the gap");
    assert_eq!(read(&s, 15, 10), Err(SourceError::Pending { offset: 15, len: 10 }));
    assert_eq!(
        read(&s, 5, 10),
        Err(SourceError::Pending { offset: 5, len: 5 }),
        "up to the next chunk"
    );
}

#[test]
fn touching_and_overlapping_chunks_read_as_one_and_held_bytes_win() {
    let s = SparseSource::new(100);
    s.supply(10, b"aaaa").unwrap();
    s.supply(20, b"cccc").unwrap();
    s.supply(14, b"bbbbbb").unwrap(); // fills 14..20 between them
    assert_eq!(read_all(&s, 10, 14), Ok(b"aaaabbbbbbcccc".to_vec()));
    s.supply(8, b"XXXXXX").unwrap(); // overlaps 10..14: only 8..10 is added, "aa" stays
    assert_eq!(read_all(&s, 8, 16), Ok(b"XXaaaabbbbbbcccc".to_vec()));
    assert_eq!(s.held(), Ok(16));
    assert_eq!(
        read(&s, 10, 14),
        Ok(b"aaaa".to_vec()),
        "a single read stops at a chunk's end"
    );
}

#[test]
fn supply_past_the_end_is_refused() {
    let s = SparseSource::new(10);
    assert_eq!(
        s.supply(8, b"abc"),
        Err(SupplyError::OutOfRange {
            offset: 8,
            len: 3,
            size: 10
        })
    );
    assert_eq!(s.held(), Ok(0));
}

#[test]
fn supply_over_the_budget_is_refused_and_adds_nothing() {
    let s = SparseSource::with_budget(100, 8);
    s.supply(0, b"abcd").unwrap();
    s.supply(2, b"cdef").unwrap(); // merged: 6 bytes
    assert_eq!(s.supply(50, b"xyz"), Err(SupplyError::Budget { needed: 9, budget: 8 }));
    assert_eq!(s.held(), Ok(6));
    assert_eq!(read(&s, 50, 1), Err(SourceError::Pending { offset: 50, len: 1 }));
}

#[test]
fn missing_lists_only_what_is_not_held_merged_and_clamped() {
    let s = SparseSource::new(100);
    s.supply(10, &[0; 10]).unwrap();
    s.supply(30, &[0; 10]).unwrap();
    assert_eq!(s.missing(&[0..50]), Ok(vec![0..10, 20..30, 40..50]));
    assert_eq!(s.missing(&[12..18, 95..200]), Ok(vec![95..100]));
    assert_eq!(s.missing(&[45..50, 40..46]), Ok(vec![40..50]), "sorted and merged");
}

/// A fetch closure over `bytes` that counts what it hands out.
struct Fetcher<'a> {
    bytes: &'a [u8],
    fetched: Cell<u64>,
    calls: RefCell<Vec<Range<u64>>>,
}

impl<'a> Fetcher<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            fetched: Cell::new(0),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn fetch(&self, r: Range<u64>) -> Result<Vec<u8>, Infallible> {
        self.fetched.set(self.fetched.get() + (r.end - r.start));
        self.calls.borrow_mut().push(r.clone());
        Ok(self.bytes[usize::try_from(r.start).unwrap()..usize::try_from(r.end).unwrap()].to_vec())
    }
}

/// Runs each read through `retry` over one source.
struct Sparse<'a> {
    source: &'a SparseSource,
    fetcher: &'a Fetcher<'a>,
    options: RetryOptions,
}

impl Step for Sparse<'_> {
    fn run<T>(&mut self, call: &mut dyn FnMut() -> laminapdf::Result<T>) -> laminapdf::Result<T> {
        retry(self.source, &self.options, |r| self.fetcher.fetch(r), call).map_err(|e| match e {
            RetryError::Lamina(e) => e,
            other => panic!("fetch or supply failed: {other:?}"),
        })
    }
}

fn sparse_facts(bytes: &[u8], options: &RetryOptions) -> (Vec<String>, u64) {
    let source = Arc::new(SparseSource::new(bytes.len() as u64));
    let fetcher = Fetcher::new(bytes);
    let facts = match open_with(&source, &OpenOptions::default(), options, |r| fetcher.fetch(r)) {
        Ok(mut doc) => facts(
            &mut doc,
            &mut Sparse {
                source: &source,
                fetcher: &fetcher,
                options: options.clone(),
            },
        ),
        Err(RetryError::Lamina(e)) => vec![show(&e)],
        Err(other) => panic!("fetch or supply failed: {other:?}"),
    };
    (facts, fetcher.fetched.get())
}

#[test]
fn every_input_reads_through_a_sparse_source_like_an_in_memory_open() {
    let mut failures = Vec::new();
    for options in [
        RetryOptions::default(),
        RetryOptions::default().with_block_size(0),
        RetryOptions::default().with_block_size(4096).with_readahead(8192),
    ] {
        for (name, bytes) in inputs() {
            let (ours, _) = sparse_facts(bytes, &options);
            let expected = in_memory_facts(bytes);
            failures.extend(first_difference(&ours, &expected).map(|d| format!("{name} ({options:?}): {d}")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn opening_a_large_intact_file_and_reading_its_pages_fetches_less_than_the_file() {
    for compact in [false, true] {
        let bytes = multi_page(compact);
        let source = Arc::new(SparseSource::new(bytes.len() as u64));
        let fetcher = Fetcher::new(bytes);
        let options = RetryOptions::default();
        let doc = open_with(&source, &OpenOptions::default(), &options, |r| fetcher.fetch(r)).unwrap();
        let count = retry(&source, &options, |r| fetcher.fetch(r), || doc.page_count()).unwrap();
        assert_eq!(count, PAGES);
        assert_eq!(source.held(), Ok(fetcher.fetched.get()), "nothing fetched twice");
        let total = fetcher.fetched.get();
        assert!(
            total * 4 < bytes.len() as u64,
            "compact {compact}: fetched {total} of {} bytes",
            bytes.len()
        );
    }
}

#[test]
fn a_fetch_is_never_more_than_the_ranges_asked_for_widened_to_blocks() {
    let bytes = multi_page(false);
    let options = RetryOptions::default().with_block_size(4096).with_readahead(0);
    let source = Arc::new(SparseSource::new(bytes.len() as u64));
    let fetcher = Fetcher::new(bytes);
    let _doc = open_with(&source, &OpenOptions::default(), &options, |r| fetcher.fetch(r)).unwrap();
    for r in fetcher.calls.borrow().iter() {
        assert_eq!(r.start % 4096, 0, "{r:?} starts on a block");
        assert!(
            r.end % 4096 == 0 || r.end == bytes.len() as u64,
            "{r:?} ends on a block or at the end"
        );
        assert!(r.end - r.start <= 2 * 64 * 1024, "{r:?} is at most a window or two");
    }
}

#[test]
fn the_loop_gives_up_with_the_last_data_pending_when_the_rounds_run_out() {
    let bytes = multi_page(false);
    let source = Arc::new(SparseSource::new(bytes.len() as u64));
    let fetcher = Fetcher::new(bytes);
    let options = RetryOptions::default().with_max_rounds(0);
    let err = open_with(&source, &OpenOptions::default(), &options, |r| fetcher.fetch(r)).unwrap_err();
    let RetryError::Lamina(e) = err else { panic!("{err:?}") };
    assert_eq!(e.code(), "data-pending");
    assert!(
        !e.pending_ranges().unwrap().is_empty(),
        "it carries the ranges still needed"
    );
    assert_eq!(fetcher.fetched.get(), 0);
}

/// Hands out `wrong` bytes for every range: the file then never reads as asked, so the reader
/// keeps asking for the same ranges.
struct Stuck(Arc<SparseSource>);

impl ByteSource for Stuck {
    fn len(&self) -> u64 {
        self.0.len()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        // Always pending, whatever was supplied.
        let _ = self.0.read_at(offset, buf);
        Err(SourceError::Pending {
            offset,
            len: buf.len() as u64,
        })
    }
}

#[test]
fn the_loop_stops_when_a_round_makes_no_progress() {
    let bytes = multi_page(false);
    let sparse = Arc::new(SparseSource::new(bytes.len() as u64));
    let stuck: Arc<dyn ByteSource> = Arc::new(Stuck(Arc::clone(&sparse)));
    let fetcher = Fetcher::new(bytes);
    let rounds = Cell::new(0);
    let err = retry(
        &sparse,
        &RetryOptions::default(),
        |r| fetcher.fetch(r),
        || {
            rounds.set(rounds.get() + 1);
            Document::open_source(Arc::clone(&stuck), OpenOptions::default())
        },
    )
    .unwrap_err();
    let RetryError::Lamina(e) = err else { panic!("{err:?}") };
    assert_eq!(e.code(), "data-pending");
    assert_eq!(rounds.get(), 2, "one fetch, then the same ranges again: stop");
}

#[test]
fn a_fetch_of_the_wrong_length_ends_the_loop() {
    let bytes = multi_page(false);
    let source = Arc::new(SparseSource::new(bytes.len() as u64));
    let err = open_with(&source, &OpenOptions::default(), &RetryOptions::default(), |r| {
        Ok::<_, Infallible>(bytes[usize::try_from(r.start).unwrap()..usize::try_from(r.end).unwrap() - 1].to_vec())
    })
    .unwrap_err();
    assert!(
        matches!(err, RetryError::FetchLength { got, expected, .. } if got + 1 == expected),
        "{err:?}"
    );
    assert_eq!(source.held(), Ok(0), "nothing supplied");
}

#[test]
fn a_fetch_error_ends_the_loop() {
    let source = Arc::new(SparseSource::new(multi_page(false).len() as u64));
    let err = open_with(&source, &OpenOptions::default(), &RetryOptions::default(), |_| {
        Err("offline")
    })
    .unwrap_err();
    assert!(matches!(err, RetryError::Fetch("offline")), "{err:?}");
}

#[test]
fn a_source_over_budget_ends_the_loop_with_a_supply_error() {
    let bytes = multi_page(false);
    let source = Arc::new(SparseSource::with_budget(bytes.len() as u64, 1024));
    let fetcher = Fetcher::new(bytes);
    let err = open_with(&source, &OpenOptions::default(), &RetryOptions::default(), |r| {
        fetcher.fetch(r)
    })
    .unwrap_err();
    assert!(matches!(err, RetryError::Supply(SupplyError::Budget { .. })), "{err:?}");
}

#[test]
fn the_source_is_shared_across_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SparseSource>();
}
