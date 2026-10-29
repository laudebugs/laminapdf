//! B2d review: reading a source without `as_slice` stays bounded on hostile input, and a
//! source that has not fetched its bytes is asked for few, small rounds.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use lamina_cos::{ByteSource, Dict, Document, Error, ObjRef, Object, OpenOptions, SourceError, Stream};
use lamina_save::Writer;

const WINDOW: usize = 64 * 1024;

/// A source without `as_slice` that counts its `read_at` calls and the bytes they return.
struct Counted {
    data: Vec<u8>,
    calls: AtomicU64,
    bytes: AtomicU64,
}

impl Counted {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            calls: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }

    fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }
}

impl ByteSource for Counted {
    fn len(&self) -> u64 {
        u64::try_from(self.data.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let tail = self.data.get(usize::try_from(offset).unwrap()..).unwrap_or_default();
        let n = tail.len().min(buf.len());
        buf[..n].copy_from_slice(&tail[..n]);
        self.bytes.fetch_add(u64::try_from(n).unwrap(), Ordering::Relaxed);
        Ok(n)
    }
}

/// Answers `Pending` outside the ranges it has fetched; records every round it is asked for.
struct Fetching {
    data: Vec<u8>,
    fetched: Mutex<Vec<bool>>,
    calls: AtomicU64,
    rounds: Mutex<Vec<Vec<(u64, u64)>>>,
}

impl Fetching {
    fn new(data: Vec<u8>) -> Self {
        let len = data.len();
        Self {
            data,
            fetched: Mutex::new(vec![false; len]),
            calls: AtomicU64::new(0),
            rounds: Mutex::new(Vec::new()),
        }
    }

    fn fetch(&self, ranges: &[(u64, u64)]) {
        self.rounds.lock().unwrap().push(ranges.to_vec());
        let mut fetched = self.fetched.lock().unwrap();
        for &(offset, len) in ranges {
            let start = usize::try_from(offset).unwrap();
            let end = (start + usize::try_from(len).unwrap()).min(self.data.len());
            fetched[start..end].iter_mut().for_each(|b| *b = true);
        }
    }

    fn rounds(&self) -> Vec<Vec<(u64, u64)>> {
        self.rounds.lock().unwrap().clone()
    }
}

impl ByteSource for Fetching {
    fn len(&self) -> u64 {
        u64::try_from(self.data.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let start = usize::try_from(offset).unwrap().min(self.data.len());
        let n = buf.len().min(self.data.len() - start);
        if !self.fetched.lock().unwrap()[start..start + n].iter().all(|b| *b) {
            return Err(SourceError::Pending {
                offset,
                len: u64::try_from(n).unwrap(),
            });
        }
        buf[..n].copy_from_slice(&self.data[start..start + n]);
        Ok(n)
    }
}

/// Opens `source`, fetching what it reports and retrying.
fn open_fetching(source: &std::sync::Arc<Fetching>) -> Document {
    for _ in 0..1000 {
        match Document::open(
            std::sync::Arc::clone(source) as std::sync::Arc<dyn ByteSource>,
            OpenOptions::default(),
        ) {
            Err(Error::DataPending { ranges }) => source.fetch(&ranges),
            other => return other.unwrap(),
        }
    }
    panic!("no progress");
}

/// A catalog, a page tree and one page, appended after `before` objects, plus `after` (more
/// objects) between the page and the catalog. Returns the bytes and the page's content ref.
fn file_with(before: impl FnOnce(&mut Writer) -> ObjRef, filler_after: usize) -> (Vec<u8>, ObjRef) {
    let mut w = Writer::new(1, 7);
    let content = before(&mut w);
    let pages = w.reserve();
    let page: Dict = [
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("Contents", content.into()),
    ]
    .into_iter()
    .collect();
    let page = w.add(page);
    if filler_after > 0 {
        w.add(Object::Stream(Stream {
            dict: Dict::new(),
            data: vec![b'.'; filler_after],
        }));
    }
    let tree: Dict = [
        ("Type", Object::name("Pages")),
        ("Kids", vec![page.into()].into()),
        ("Count", 1.into()),
    ]
    .into_iter()
    .collect();
    w.set(pages, tree);
    let catalog: Dict = [("Type", Object::name("Catalog")), ("Pages", pages.into())]
        .into_iter()
        .collect();
    let root = w.add(catalog);
    (w.finish(&[("Root", root)].into_iter().collect()).unwrap(), content)
}

#[test]
fn a_length_past_the_end_of_the_input_is_recovered_without_reading_to_the_end() {
    // Object 1's raw body lies: `/Length` says a billion bytes, the data is 5.
    let (bytes, content) = file_with(
        |w| w.add_raw(b"<< /Length 1000000000 >>\nstream\nhello\nendstream".to_vec()),
        2 * 1024 * 1024,
    );
    let source = std::sync::Arc::new(Counted::new(bytes.clone()));
    let doc = Document::open(
        std::sync::Arc::clone(&source) as std::sync::Arc<dyn ByteSource>,
        OpenOptions::default(),
    )
    .unwrap();
    let before = source.bytes();
    let object = doc.resolve(content).unwrap();
    let read = source.bytes() - before;
    let in_memory = Document::open(bytes, OpenOptions::default())
        .unwrap()
        .resolve(content)
        .unwrap();
    assert!(
        object == in_memory && read <= 4 * u64::try_from(WINDOW).unwrap(),
        "read {read} bytes to resolve a 5-byte stream"
    );
}

#[test]
fn a_length_past_the_window_but_wrong_is_checked_with_a_probe_not_a_grown_window() {
    // `/Length` points 1 MiB further into the file (at filler), where there is no `endstream`.
    let (bytes, content) = file_with(
        |w| w.add_raw(b"<< /Length 1048576 >>\nstream\nhello\nendstream".to_vec()),
        2 * 1024 * 1024,
    );
    let source = std::sync::Arc::new(Counted::new(bytes.clone()));
    let doc = Document::open(
        std::sync::Arc::clone(&source) as std::sync::Arc<dyn ByteSource>,
        OpenOptions::default(),
    )
    .unwrap();
    let before = source.bytes();
    let object = doc.resolve(content).unwrap();
    let read = source.bytes() - before;
    let in_memory = Document::open(bytes, OpenOptions::default())
        .unwrap()
        .resolve(content)
        .unwrap();
    assert!(
        object == in_memory && read <= 4 * u64::try_from(WINDOW).unwrap(),
        "read {read} bytes to resolve a 5-byte stream"
    );
}

/// Claims a terabyte; serves a broken PDF (no `startxref`) at the start and end, and answers
/// `Pending` everywhere else.
struct HugeAndPending {
    calls: AtomicU64,
}

const HUGE: u64 = 1 << 40;

impl ByteSource for HugeAndPending {
    fn len(&self) -> u64 {
        HUGE
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let window = u64::try_from(WINDOW).unwrap();
        if offset < window || offset >= HUGE - window {
            buf.fill(b' ');
            return Ok(buf.len());
        }
        Err(SourceError::Pending {
            offset,
            len: u64::try_from(buf.len()).unwrap(),
        })
    }
}

#[test]
fn a_source_whose_length_lies_is_asked_a_bounded_number_of_times() {
    let source = std::sync::Arc::new(HugeAndPending {
        calls: AtomicU64::new(0),
    });
    let result = Document::open(
        std::sync::Arc::clone(&source) as std::sync::Arc<dyn ByteSource>,
        OpenOptions::default(),
    );
    let calls = source.calls.load(Ordering::Relaxed);
    let Err(Error::DataPending { ranges }) = result else {
        panic!("expected DataPending, got {result:?}");
    };
    let asked: u64 = ranges.iter().map(|r| r.1).sum();
    assert!(
        calls <= 2048 && asked <= 64 << 20,
        "{calls} read_at calls, {asked} bytes asked for in one round"
    );
}

#[test]
fn a_startxref_across_the_last_window_boundary_needs_one_round_and_never_the_whole_file() {
    // Pad the end of the file so that `startxref` starts 4 bytes before the last window.
    let (mut bytes, _) = file_with(
        |w| {
            w.add(Object::Stream(Stream {
                dict: Dict::new(),
                data: vec![b'.'; 300_000],
            }))
        },
        0,
    );
    let at = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
    let tail = bytes.split_off(at);
    let pad = (WINDOW - 4 + WINDOW * (at / WINDOW + 1)) - at;
    bytes.extend(std::iter::repeat_n(b'\n', pad));
    bytes.extend(tail);
    let at = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
    assert!(at % WINDOW == WINDOW - 4 && at / WINDOW + 1 == (bytes.len() - 1) / WINDOW);
    let len = u64::try_from(bytes.len()).unwrap();
    let source = std::sync::Arc::new(Fetching::new(bytes));
    open_fetching(&source);
    let rounds = source.rounds();
    let largest: u64 = rounds.iter().map(|r| r.iter().map(|x| x.1).sum::<u64>()).max().unwrap();
    // Round 1: head and tail (which hold `startxref`); round 2: the xref entries' headers.
    assert!(
        rounds.len() <= 2 && largest < len / 2,
        "{} rounds, largest {largest} of {len} bytes: {rounds:?}",
        rounds.len()
    );
}

#[test]
fn the_xref_entry_probes_of_many_small_objects_are_asked_for_in_few_ranges() {
    // 2 000 small objects between a 512 KiB stream and a 1 MiB stream, so none of them is in
    // the head or tail window.
    let mut w = Writer::new(1, 7);
    w.add(Object::Stream(Stream {
        dict: Dict::new(),
        data: vec![b'.'; 512 * 1024],
    }));
    let small: Vec<Object> = (0..2000).map(|i| w.add(Object::Integer(i)).into()).collect();
    w.add(Object::Stream(Stream {
        dict: Dict::new(),
        data: vec![b'.'; 1024 * 1024],
    }));
    let pages = w.reserve();
    let tree: Dict = [
        ("Type", Object::name("Pages")),
        ("Kids", Vec::<Object>::new().into()),
        ("Count", 0.into()),
        ("Small", small.into()),
    ]
    .into_iter()
    .collect();
    w.set(pages, tree);
    let catalog: Dict = [("Type", Object::name("Catalog")), ("Pages", pages.into())]
        .into_iter()
        .collect();
    let root = w.add(catalog);
    let bytes = w.finish(&[("Root", root)].into_iter().collect()).unwrap();
    let source = std::sync::Arc::new(Fetching::new(bytes));
    open_fetching(&source);
    let ranges: usize = source.rounds().iter().map(Vec::len).sum();
    let calls = source.calls.load(Ordering::Relaxed);
    assert!(ranges <= 16 && calls <= 200, "{ranges} ranges, {calls} read_at calls");
}

/// A file whose object 1 is a stream with no `endstream` anywhere after it, followed by 3 MiB
/// of data.
fn stream_without_endstream() -> (Vec<u8>, ObjRef) {
    let mut body = b"<< >>\nstream\n".to_vec();
    body.extend(std::iter::repeat_n(b'.', 3 * 1024 * 1024));
    file_with(|w| w.add_raw(body), 0)
}

#[test]
fn a_missing_endstream_is_searched_for_up_to_max_read_window_only() {
    let (bytes, content) = stream_without_endstream();
    let limits = lamina_cos::Limits::default().with_max_read_window(512 * 1024);
    let source = std::sync::Arc::new(Counted::new(bytes));
    let doc = Document::open(
        std::sync::Arc::clone(&source) as std::sync::Arc<dyn ByteSource>,
        OpenOptions::default().with_limits(limits),
    )
    .unwrap();
    let before = source.bytes();
    let result = doc.resolve(content);
    let read = source.bytes() - before;
    assert!(
        matches!(
            result,
            Err(Error::LimitExceeded {
                limit: lamina_cos::Limit::ReadWindow,
                ..
            })
        ) && read <= 1024 * 1024,
        "{result:?} after reading {read} bytes"
    );
}

#[test]
fn a_missing_endstream_within_max_read_window_fails_as_in_memory() {
    let (bytes, content) = stream_without_endstream();
    let windowed = Document::open(Counted::new(bytes.clone()), OpenOptions::default())
        .unwrap()
        .resolve(content);
    let in_memory = Document::open(bytes, OpenOptions::default()).unwrap().resolve(content);
    assert!(
        windowed.is_err() && windowed == in_memory,
        "{windowed:?} vs {in_memory:?}"
    );
}
