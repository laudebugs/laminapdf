//! The `content_append_scan` fuzz target's entry point (todo E2e). `fuzz/fuzz_targets/
//! content_append_scan.rs` calls it (through `fuzz/src/lib.rs`, which includes this file), and
//! `tests/append.rs` replays every saved crash input (`fixtures/regressions/content_append_scan/`)
//! through it.

use laminapdf_content::{Limits, PageContentWriter, Wrapped, scan_balance};
use laminapdf_cos::{Dict, Object, Stream};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_save::test_support::Writer;

/// The most streams the input is split into.
const MAX_STREAMS: usize = 8;

/// Splits `data` at its zero bytes into at most [`MAX_STREAMS`] content streams and checks the
/// scan and a wrap of a page holding them. Invariants: no panic; every unmatched `Q` the scan
/// reports is a `Q` byte in its stream; blanking them leaves the depth and text state alone and
/// nothing unmatched; a wrap either refuses with `invalid-structure` and writes nothing, or
/// leaves the page wrapped, so that a second wrap does nothing, and changes only the streams
/// that held an unmatched `Q`; rolling it back empties the overlay.
pub fn content_append_scan(data: &[u8]) {
    let streams: Vec<&[u8]> = data.splitn(MAX_STREAMS, |&b| b == 0).collect();
    let limits = Limits::default();
    let Ok(balance) = scan_balance(&streams, &limits) else {
        return;
    };
    let mut blanked: Vec<Vec<u8>> = streams.iter().map(|s| s.to_vec()).collect();
    for at in &balance.unmatched {
        let byte = blanked.get_mut(at.stream).and_then(|s| s.get_mut(at.offset));
        assert_eq!(byte.as_deref(), Some(&b'Q'), "an unmatched Q at {at:?}");
        if let Some(byte) = byte {
            *byte = b' ';
        }
    }
    let views: Vec<&[u8]> = blanked.iter().map(Vec::as_slice).collect();
    let again = scan_balance(&views, &limits).unwrap();
    assert!(again.unmatched.is_empty(), "blanking leaves nothing unmatched");
    assert_eq!((again.depth, again.in_text), (balance.depth, balance.in_text));

    let mut doc = Document::open(page_file(&streams), OpenOptions::default()).unwrap();
    let page = doc.pages().unwrap().get(0).unwrap().cos().clone();
    let before = decoded(&doc);
    let mark = doc.cos().journal_mark();
    let result = PageContentWriter::new(doc.cos_mut().edit(), &page).and_then(|mut w| w.wrap_existing());
    match result {
        Err(e) => {
            assert_eq!(e.code(), "invalid-structure", "{e}");
            assert!(!doc.cos().is_dirty(), "a refused wrap writes nothing");
        }
        Ok(Wrapped::Already) => assert!(balance.wrapped),
        Ok(Wrapped::Now { blanked: at, .. }) => {
            assert_eq!(at, balance.unmatched);
            let after = decoded(&doc);
            assert_eq!(after.len(), before.len() + 2);
            for (i, (old, new)) in before.iter().zip(&after[1..]).enumerate() {
                if at.iter().all(|a| a.stream != i) {
                    assert_eq!(old, new, "stream {i} without an unmatched Q is unchanged");
                }
            }
            let page = doc.pages().unwrap().get(0).unwrap().cos().clone();
            let second = PageContentWriter::new(doc.cos_mut().edit(), &page).and_then(|mut w| w.wrap_existing());
            assert_eq!(second.unwrap(), Wrapped::Already, "a wrapped page isn't wrapped again");
        }
        Ok(other) => panic!("unknown result {other:?}"),
    }
    doc.cos_mut().rollback_to(mark).unwrap();
    assert!(doc.cos().overlay().is_empty(), "the rollback empties the overlay");
}

/// The page's streams, decoded.
fn decoded(doc: &Document) -> Vec<Vec<u8>> {
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let Some(Object::Array(items)) = page.cos().dict.get("Contents").cloned() else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| match &*doc.deref(item).unwrap() {
            Object::Stream(s) => doc.decode_stream(s).unwrap(),
            _ => Vec::new(),
        })
        .collect()
}

/// A one-page file whose `/Contents` array holds `streams`, the second one Flate-compressed.
fn page_file(streams: &[&[u8]]) -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let contents: Vec<Object> = streams
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let stream = if i == 1 {
                Stream::flate(Dict::new(), s)
            } else {
                Stream::new(Dict::new(), s.to_vec())
            };
            w.add(stream).into()
        })
        .collect();
    let mut page = Dict::new();
    page.insert("Type", Object::name("Page"));
    page.insert("Parent", pages);
    page.insert("MediaBox", vec![0.into(), 0.into(), 100.into(), 100.into()]);
    page.insert("Contents", contents);
    let page = w.add(page);
    let mut tree = Dict::new();
    tree.insert("Type", Object::name("Pages"));
    tree.insert("Kids", vec![page.into()]);
    tree.insert("Count", 1);
    w.set(pages, tree);
    let mut catalog = Dict::new();
    catalog.insert("Type", Object::name("Catalog"));
    catalog.insert("Pages", pages);
    let root = w.add(catalog);
    w.finish(&[("Root", root)].into_iter().collect()).unwrap()
}
