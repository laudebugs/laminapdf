//! ADR 0005 §2, §7: `Document` owns its input through a `ByteSource`, is `Send`, and several
//! documents can share one source. A source without `as_slice` is read in windows (B2d): it
//! gives the same results as an in-memory one over the whole corpus and the encrypted
//! fixtures, and a source that hasn't fetched its bytes yet is asked only for the windows the
//! reader needs.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use common::{corpus, fixture, fixtures_dir};
use laminapdf_cos::{
    ByteSource, Dict, Document, Error, Limit, Limits, ObjRef, Object, OpenOptions, PdfString, Source, SourceError,
    Stream,
};
use laminapdf_save::Writer;

/// A source that is not in memory (no `as_slice`): `read_at` only, serving at most `max`
/// bytes per call, and counting its calls.
struct Chunked {
    data: Vec<u8>,
    max: usize,
    calls: AtomicU64,
    bytes: AtomicU64,
}

impl Chunked {
    fn new(data: Vec<u8>, max: usize) -> Self {
        Self {
            data,
            max,
            calls: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }
}

/// A [`Chunked`] source serving reads of at most 4 KiB: what the windowed tests use.
fn windowed(data: Vec<u8>) -> Chunked {
    Chunked::new(data, 4096)
}

impl ByteSource for Chunked {
    fn len(&self) -> u64 {
        u64::try_from(self.data.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let tail = self.data.get(usize::try_from(offset).unwrap()..).unwrap_or_default();
        let n = tail.len().min(buf.len()).min(self.max);
        buf[..n].copy_from_slice(&tail[..n]);
        self.bytes.fetch_add(u64::try_from(n).unwrap(), Ordering::Relaxed);
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

fn page_count(doc: &laminapdf_cos::Result<Document>) -> Option<usize> {
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
    let doc = Document::open(Chunked::new(hello(), 7), OpenOptions::default()).unwrap();
    assert_eq!((doc.page_count(), doc.data()), (Ok(1), None));
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
    assert!(Arc::ptr_eq(a.source(), b.source()) && std::ptr::eq(a.data().unwrap(), b.data().unwrap()));
}

#[test]
fn in_memory_input_is_parsed_in_place() {
    let source: Arc<dyn ByteSource> = Arc::new(hello());
    let doc = Document::open(Arc::clone(&source), OpenOptions::default()).unwrap();
    assert!(std::ptr::eq(doc.data().unwrap(), source.as_slice().unwrap()));
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

// ---------------------------------------------------------------------------------------------
// Windowed reading (B2d)
// ---------------------------------------------------------------------------------------------

/// A source that answers `Pending` for any range it has not "fetched", like an HTTP range
/// cache; [`Fetching::fetch`] fetches ranges and counts the bytes.
struct Fetching {
    data: Vec<u8>,
    fetched: Mutex<Vec<bool>>,
    bytes_fetched: AtomicU64,
    /// Every `DataPending` the reader reported, in order.
    rounds: Mutex<Vec<Vec<(u64, u64)>>>,
}

impl Fetching {
    fn new(data: Vec<u8>) -> Arc<Self> {
        let len = data.len();
        Arc::new(Self {
            data,
            fetched: Mutex::new(vec![false; len]),
            bytes_fetched: AtomicU64::new(0),
            rounds: Mutex::new(Vec::new()),
        })
    }

    /// Fetches `ranges`. Panics when a range is entirely fetched already: the reader asked
    /// again for bytes it was given (and would loop forever if nothing else were new). A range
    /// may still overlap fetched bytes: a window the reader needs whole can hold a few bytes an
    /// earlier probe fetched, and this source answers `Pending` for the whole of any read
    /// that is not complete.
    fn fetch(&self, ranges: &[(u64, u64)]) {
        self.rounds.lock().unwrap().push(ranges.to_vec());
        let mut fetched = self.fetched.lock().unwrap();
        let mut new = 0;
        for &(offset, len) in ranges {
            let start = usize::try_from(offset).unwrap();
            let end = start + usize::try_from(len).unwrap();
            let mut new_here = 0;
            for byte in &mut fetched[start..end.min(self.data.len())] {
                new_here += u64::from(!*byte);
                *byte = true;
            }
            assert!(
                new_here > 0,
                "DataPending for a range already fetched: ({offset}, {len}) in {ranges:?}"
            );
            new += new_here;
        }
        self.bytes_fetched.fetch_add(new, Ordering::Relaxed);
    }

    fn bytes_fetched(&self) -> u64 {
        self.bytes_fetched.load(Ordering::Relaxed)
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

/// Runs `step`, fetching what it reports pending and retrying, as a shell would.
fn retry<T>(source: Option<&Fetching>, mut step: impl FnMut() -> laminapdf_cos::Result<T>) -> laminapdf_cos::Result<T> {
    for _ in 0..10_000 {
        match (step(), source) {
            (Err(Error::DataPending { ranges }), Some(source)) => source.fetch(&ranges),
            (result, _) => return result,
        }
    }
    panic!("no progress after 10 000 rounds");
}

/// `result`, unless it is `DataPending`: that ends the step so it is retried.
fn settled<T>(result: laminapdf_cos::Result<T>) -> laminapdf_cos::Result<laminapdf_cos::Result<T>> {
    match result {
        Err(e @ Error::DataPending { .. }) => Err(e),
        other => Ok(other),
    }
}

fn show<T: Debug>(result: &T) -> String {
    format!("{result:?}")
}

/// Everything a reader can tell about `doc`, as text: structure, every object and decoded
/// stream, the pages and their content, then the warnings raised along the way.
fn facts(doc: &Document, source: Option<&Fetching>) -> Vec<String> {
    let mut out = vec![show(&(
        doc.header_version(),
        doc.revisions(),
        doc.was_repaired(),
        doc.has_xref_stream(),
        doc.startxref(),
        doc.trailer(),
        doc.encryption(),
        doc.permissions(),
        doc.matched_password(),
    ))];
    let numbers: Vec<u32> = doc.object_numbers().collect();
    for num in numbers {
        out.push(show(&retry(source, || {
            let object = settled(doc.object(num))?;
            let decoded = match object.as_deref() {
                Ok(Object::Stream(s)) => Some(settled(doc.decode_stream(s))?),
                _ => None,
            };
            Ok((num, object, decoded))
        })));
    }
    out.push(show(&retry(source, || {
        let pages = doc.pages()?;
        let content = pages
            .iter()
            .map(|page| settled(doc.page_content(page)))
            .collect::<laminapdf_cos::Result<Vec<_>>>()?;
        Ok((pages, content))
    })));
    out.push(show(&doc.warnings()));
    out
}

/// The first line where `ours` and `theirs` differ, for a readable failure.
fn first_difference(ours: &[String], theirs: &[String]) -> Option<String> {
    if ours == theirs {
        return None;
    }
    let short = |s: &str| s.chars().take(300).collect::<String>();
    let at = ours
        .iter()
        .zip(theirs)
        .position(|(a, b)| a != b)
        .unwrap_or(ours.len().min(theirs.len()));
    Some(format!(
        "line {at}: windowed {:?} vs in memory {:?} ({} vs {} lines)",
        ours.get(at).map(|s| short(s)),
        theirs.get(at).map(|s| short(s)),
        ours.len(),
        theirs.len()
    ))
}

/// `bytes` with its last `startxref` keyword broken, so opening it means repair.
fn without_startxref(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let at = out.windows(9).rposition(|w| w == b"startxref").unwrap();
    out[at] = b'S';
    out
}

/// `(name, bytes, options)` for every corpus file and every encrypted fixture (opened with its
/// user password), each encrypted one also with its xref broken (repaired, decrypting its
/// object streams on the way).
fn inputs() -> &'static [(String, Vec<u8>, OpenOptions)] {
    static INPUTS: OnceLock<Vec<(String, Vec<u8>, OpenOptions)>> = OnceLock::new();
    INPUTS.get_or_init(build_inputs)
}

fn build_inputs() -> Vec<(String, Vec<u8>, OpenOptions)> {
    let passwords: serde_json::Value = serde_json::from_slice(&fixture("encrypted/passwords.json")).unwrap();
    let mut out: Vec<_> = corpus()
        .into_iter()
        .map(|(name, bytes)| (name, bytes, OpenOptions::default()))
        .collect();
    let mut encrypted: Vec<_> = std::fs::read_dir(fixtures_dir().join("encrypted"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
        .collect();
    encrypted.sort();
    for path in encrypted {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(&path).unwrap();
        // The extra fixtures that passwords.json does not list use `user-pw` when they have a
        // user password.
        let options = match passwords["files"][&name]["user"].as_str() {
            Some(user) => OpenOptions::default().with_password(user),
            None if Document::open(bytes.clone(), OpenOptions::default()).err() == Some(Error::PasswordRequired) => {
                OpenOptions::default().with_password("user-pw")
            }
            None => OpenOptions::default(),
        };
        out.push((
            format!("encrypted/{name} (repaired)"),
            without_startxref(&bytes),
            options.clone(),
        ));
        out.push((format!("encrypted/{name}"), bytes, options));
    }
    out.extend(large_inputs());
    out
}

/// Files bigger than one window, which the fixtures don't have: a multi-page file of about
/// 1.3 MiB (classic xref and compact), and a repaired one with an object header across the
/// 64 KiB window boundary.
fn large_inputs() -> Vec<(String, Vec<u8>, OpenOptions)> {
    vec![
        (
            "generated/classic".into(),
            multi_page(false).to_vec(),
            OpenOptions::default(),
        ),
        (
            "generated/compact".into(),
            multi_page(true).to_vec(),
            OpenOptions::default(),
        ),
        ("generated/repaired".into(), straddling_header(), OpenOptions::default()),
    ]
}

const PAGES: usize = 32;

/// `PAGES` pages, each with a 40 KiB content stream; the content streams come first, so the
/// page tree, the catalog and the xref sit together at the end, as many writers lay them out.
fn multi_page(compact: bool) -> &'static [u8] {
    static FILES: OnceLock<[Vec<u8>; 2]> = OnceLock::new();
    &FILES.get_or_init(|| [build_multi_page(false), build_multi_page(true)])[usize::from(compact)]
}

fn build_multi_page(compact: bool) -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let contents: Vec<ObjRef> = (0..PAGES)
        .map(|i| {
            let line = format!("BT /F1 12 Tf 72 720 Td (page {i}) Tj ET\n");
            let data = line.repeat(40 * 1024 / line.len()).into_bytes();
            w.add(Object::Stream(Stream {
                dict: Dict::new(),
                data,
            }))
        })
        .collect();
    let pages = w.reserve();
    let kids: Vec<Object> = contents
        .iter()
        .map(|&content| {
            let page: Dict = [
                ("Type", Object::name("Page")),
                ("Parent", pages.into()),
                ("Contents", content.into()),
            ]
            .into_iter()
            .collect();
            w.add(page).into()
        })
        .collect();
    let count = i64::try_from(kids.len()).unwrap();
    let tree: Dict = [
        ("Type", Object::name("Pages")),
        ("Kids", kids.into()),
        ("Count", count.into()),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
    ]
    .into_iter()
    .collect();
    w.set(pages, tree);
    let catalog: Dict = [("Type", Object::name("Catalog")), ("Pages", pages.into())]
        .into_iter()
        .collect();
    let root = w.add(catalog);
    let trailer: Dict = [("Root", root)].into_iter().collect();
    let bytes = if compact {
        w.finish_compact(&trailer)
    } else {
        w.finish(&trailer)
    };
    bytes.unwrap()
}

/// A file without xref (so it is repaired), five windows long, whose object 3's `3 0 obj`
/// header starts before offset 65 536 and ends after it.
fn straddling_header() -> Vec<u8> {
    let mut out = b"%PDF-1.7\n1 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec();
    let filler = |num: u32, len: usize| format!("{num} 0 obj\n({})\nendobj\n", "x".repeat(len));
    out.extend_from_slice(filler(2, 60_000).as_bytes());
    // Pad so that `3 0 obj` runs across the boundary: `3 0` before it, `obj` after.
    let header_at = 64 * 1024 - 3;
    assert!(out.len() < header_at);
    out.resize(header_at, b' ');
    out.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 1 0 R >>\nendobj\n");
    out.extend_from_slice(b"4 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n");
    out.extend_from_slice(filler(5, 200_000).as_bytes());
    out.extend_from_slice(b"trailer\n<< /Root 4 0 R >>\n%%EOF\n");
    out
}

fn in_memory_facts(bytes: &[u8], options: &OpenOptions) -> Vec<String> {
    match Document::open(bytes.to_vec(), options.clone()) {
        Ok(doc) => facts(&doc, None),
        Err(e) => vec![show(&e)],
    }
}

#[test]
fn a_source_without_as_slice_reads_like_an_in_memory_one_for_every_input() {
    let mut failures = Vec::new();
    for (name, bytes, options) in inputs() {
        let (bytes, options) = (bytes.clone(), options.clone());
        let expected = in_memory_facts(&bytes, &options);
        let ours = match Document::open(windowed(bytes.clone()), options) {
            Ok(doc) => facts(&doc, None),
            Err(e) => vec![show(&e)],
        };
        failures.extend(first_difference(&ours, &expected).map(|d| format!("{name}: {d}")));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Opens `source`, fetching and retrying until the open no longer reports `DataPending`.
fn open_fetching(source: &Arc<Fetching>, options: &OpenOptions) -> laminapdf_cos::Result<Document> {
    retry(Some(source), || {
        Document::open(Arc::clone(source) as Arc<dyn ByteSource>, options.clone())
    })
}

#[test]
fn a_pending_source_driven_by_a_retry_loop_reads_like_an_in_memory_one_for_every_input() {
    let mut failures = Vec::new();
    for (name, bytes, options) in inputs() {
        let (bytes, options) = (bytes.clone(), options.clone());
        let expected = in_memory_facts(&bytes, &options);
        let source = Fetching::new(bytes.clone());
        let ours = match open_fetching(&source, &options) {
            Ok(doc) => facts(&doc, Some(&source)),
            Err(e) => vec![show(&e)],
        };
        failures.extend(first_difference(&ours, &expected).map(|d| format!("{name}: {d}")));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn opening_an_intact_large_file_and_walking_its_pages_fetches_less_than_the_file() {
    for compact in [false, true] {
        let bytes = multi_page(compact).to_vec();
        assert!(bytes.len() > 1024 * 1024);
        let source = Fetching::new(bytes.clone());
        let doc = open_fetching(&source, &OpenOptions::default()).unwrap();
        assert_eq!(retry(Some(&source), || doc.page_count()), Ok(PAGES));
        let fetched = source.bytes_fetched();
        assert!(
            fetched * 4 < u64::try_from(bytes.len()).unwrap(),
            "compact {compact}: fetched {fetched} of {} bytes",
            bytes.len()
        );
    }
}

#[test]
fn data_pending_never_asks_for_the_whole_of_an_intact_large_file() {
    for compact in [false, true] {
        let bytes = multi_page(compact).to_vec();
        let len = u64::try_from(bytes.len()).unwrap();
        let source = Fetching::new(bytes);
        let doc = open_fetching(&source, &OpenOptions::default()).unwrap();
        facts(&doc, Some(&source));
        let largest = source.rounds().iter().flatten().map(|&(_, n)| n).max().unwrap();
        assert!(
            largest <= 64 * 1024,
            "compact {compact}: asked for {largest} of {len} bytes at once"
        );
    }
}

#[test]
fn the_first_data_pending_asks_for_the_head_and_the_tail_window_together() {
    let bytes = multi_page(false).to_vec();
    let len = u64::try_from(bytes.len()).unwrap();
    let tail = (len - 1) / (64 * 1024) * (64 * 1024);
    let err = Document::open(Fetching::new(bytes) as Arc<dyn ByteSource>, OpenOptions::default()).err();
    assert_eq!(
        err,
        Some(Error::DataPending {
            ranges: vec![(0, 64 * 1024), (tail, len - tail)]
        })
    );
}

#[test]
fn a_lazy_call_that_hits_missing_data_reports_it_and_succeeds_on_retry() {
    let bytes = multi_page(false).to_vec();
    let source = Fetching::new(bytes.clone());
    let doc = open_fetching(&source, &OpenOptions::default()).unwrap();
    let pages = retry(Some(&source), || doc.pages()).unwrap();
    // The first content stream is in the first window, which open fetched; the 20th is not.
    let pending = doc.page_content(&pages[20]);
    let Err(Error::DataPending { ranges }) = pending else {
        panic!("expected DataPending, got {pending:?}");
    };
    assert!(ranges.iter().all(|&(_, n)| n <= 64 * 1024), "{ranges:?}");
    source.fetch(&ranges);
    let content = retry(Some(&source), || doc.page_content(&pages[20])).unwrap();
    let in_memory = Document::open(bytes, OpenOptions::default()).unwrap();
    assert_eq!(
        (content, doc.page_count()),
        (
            in_memory.page_content(&in_memory.pages().unwrap()[20]).unwrap(),
            Ok(PAGES)
        )
    );
}

#[test]
fn repair_on_a_windowed_source_finds_a_header_across_the_window_boundary() {
    let bytes = straddling_header();
    let header = bytes.windows(7).position(|w| w == b"3 0 obj").unwrap();
    assert!(header < 64 * 1024 && header + 7 > 64 * 1024);
    let doc = Document::open(windowed(bytes), OpenOptions::default()).unwrap();
    let page = doc.resolve(ObjRef::new(3, 0)).unwrap();
    assert_eq!(
        (doc.was_repaired(), page.as_dict().and_then(|d| d.get_name("Type"))),
        (true, Some(&b"Page"[..]))
    );
}

#[test]
fn repair_reports_every_missing_window_at_once_after_the_startxref_search() {
    let bytes = straddling_header();
    let len = u64::try_from(bytes.len()).unwrap();
    let source = Fetching::new(bytes);
    let doc = open_fetching(&source, &OpenOptions::default()).unwrap();
    let window = 64 * 1024;
    let tail = (len - 1) / window * window;
    // Head and tail first; then the backward search for `startxref` asks for as many more
    // windows as it has read (one); then repair's scan asks for every window still missing in
    // one round.
    assert_eq!(
        (doc.page_count(), source.rounds()),
        (
            Ok(1),
            vec![
                vec![(0, window), (tail, len - tail)],
                vec![(tail - window, window)],
                vec![(window, tail - 2 * window)],
            ]
        )
    );
}

/// One page whose `/Big` entry refers to an indirect `len`-byte literal string (object 2),
/// which runs past the first window.
fn long_string_file(len: usize) -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let big = w.add(Object::String(PdfString::literal("y".repeat(len))));
    let page: Dict = [
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("Big", big.into()),
    ]
    .into_iter()
    .collect();
    let page = w.add(page);
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
    w.finish(&[("Root", root)].into_iter().collect()).unwrap()
}

fn resolve_both(bytes: &[u8], limits: Limits, r: ObjRef) -> (String, String) {
    let options = OpenOptions::default().with_limits(limits);
    let windowed = Document::open(windowed(bytes.to_vec()), options.clone()).unwrap();
    let in_memory = Document::open(bytes.to_vec(), options).unwrap();
    (show(&windowed.resolve(r)), show(&in_memory.resolve(r)))
}

#[test]
fn a_window_grows_past_64_kib_for_an_object_within_the_limits() {
    let bytes = long_string_file(150_000);
    let limits = Limits::default().with_max_string_length(200_000);
    let (windowed, in_memory) = resolve_both(&bytes, limits, ObjRef::new(2, 0));
    assert!(
        windowed == in_memory && windowed.starts_with("Ok"),
        "{}",
        windowed.chars().take(100).collect::<String>()
    );
}

#[test]
fn an_object_over_a_limit_fails_at_the_same_offset_as_in_memory() {
    let bytes = long_string_file(150_000);
    let limits = Limits::default().with_max_string_length(100_000);
    let (windowed, in_memory) = resolve_both(&bytes, limits, ObjRef::new(2, 0));
    let expected_kind = show(&Limit::StringLength);
    assert!(
        windowed == in_memory && windowed.contains(&expected_kind),
        "{windowed} vs {in_memory}"
    );
}

#[test]
fn documents_over_one_windowed_source_share_its_cache() {
    let source = Arc::new(Chunked::new(multi_page(false).to_vec(), usize::MAX));
    let shared = Source::from(Arc::clone(&source) as Arc<dyn ByteSource>);
    let a = Document::open(shared.clone(), OpenOptions::default()).unwrap();
    assert_eq!(a.page_count(), Ok(PAGES));
    let after_one = source.bytes.load(Ordering::Relaxed);
    let b = Document::open(shared, OpenOptions::default()).unwrap();
    assert_eq!(b.page_count(), Ok(PAGES));
    // The second document reads no window again, only the small xref entry probes (which are
    // never cached): far less than one window.
    let again = source.bytes.load(Ordering::Relaxed) - after_one;
    assert!(
        again < 64 * 1024 / 4,
        "the second document read {again} bytes (first: {after_one})"
    );
}

#[test]
fn a_windowed_source_has_no_in_memory_data_but_updates_incrementally() {
    let bytes = multi_page(false).to_vec();
    let windowed = Document::open(windowed(bytes.clone()), OpenOptions::default()).unwrap();
    let in_memory = Document::open(bytes, OpenOptions::default()).unwrap();
    let change = [(ObjRef::new(999, 0), Object::Integer(1))];
    let eol = laminapdf_save::XrefEol::CrLf;
    assert_eq!(
        (
            windowed.data(),
            laminapdf_save::incremental_update(&windowed, &change, eol)
        ),
        (None, laminapdf_save::incremental_update(&in_memory, &change, eol))
    );
}
