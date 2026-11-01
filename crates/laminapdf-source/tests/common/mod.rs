//! Shared by the integration tests: inputs and what a reader can tell about a document.
#![allow(dead_code, unreachable_pub, reason = "each test file uses part of this module")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests"
)]

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::OnceLock;

use laminapdf::{CancelToken, Document, SaveMode, SaveOptions};
use laminapdf_cos::{Dict, ObjRef, Object, Stream};
use laminapdf_save::Writer;

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// `(name, bytes)` for every corpus file, sorted, then the generated large files.
pub fn inputs() -> &'static [(String, Vec<u8>)] {
    static INPUTS: OnceLock<Vec<(String, Vec<u8>)>> = OnceLock::new();
    INPUTS.get_or_init(|| {
        let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(fixtures_dir().join("corpus"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
            .map(|p| {
                (
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read(&p).unwrap(),
                )
            })
            .collect();
        files.sort();
        files.push(("generated/classic".into(), multi_page(false).to_vec()));
        files.push(("generated/compact".into(), multi_page(true).to_vec()));
        files
    })
}

pub const PAGES: u32 = 32;

/// `PAGES` pages, each with a 40 KiB content stream first, then the page tree, catalog and
/// xref together at the end (about 1.3 MiB), as many writers lay them out.
pub fn multi_page(compact: bool) -> &'static [u8] {
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
    let tree: Dict = [
        ("Type", Object::name("Pages")),
        ("Kids", kids.into()),
        ("Count", i64::from(PAGES).into()),
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
    if compact {
        w.finish_compact(&trailer)
    } else {
        w.finish(&trailer)
    }
    .unwrap()
}

pub fn show<T: Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// How a test runs one read: once (in memory), or through a fetch-and-retry loop.
pub trait Step {
    fn run<T>(&mut self, call: &mut dyn FnMut() -> laminapdf::Result<T>) -> laminapdf::Result<T>;
}

/// Runs each read once.
pub struct Once;

impl Step for Once {
    fn run<T>(&mut self, call: &mut dyn FnMut() -> laminapdf::Result<T>) -> laminapdf::Result<T> {
        call()
    }
}

/// What a reader can tell about `doc`: pages, page facts, Info, a full-rewrite save (which reads
/// and decodes every object), then the warnings raised along the way.
pub fn facts(doc: &mut Document, step: &mut impl Step) -> Vec<String> {
    let mut out = vec![
        show(&step.run(&mut || doc.page_count())),
        show(&step.run(&mut || doc.page_infos())),
        show(&step.run(&mut || doc.info())),
    ];
    let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    let saved = step.run(&mut || doc.save(&options, &CancelToken::new()).map(|s| s.bytes));
    out.push(show(&saved.map(|b| (b.len(), b))));
    out.push(show(&doc.warnings().next_batch(u32::MAX)));
    out
}

/// The facts of an in-memory open of `bytes`, or its error.
pub fn in_memory_facts(bytes: &[u8]) -> Vec<String> {
    match Document::open_bytes(bytes.to_vec(), laminapdf::OpenOptions::default()) {
        Ok(mut doc) => facts(&mut doc, &mut Once),
        Err(e) => vec![show(&e)],
    }
}

/// The first line where `ours` and `theirs` differ, for a readable failure.
pub fn first_difference(ours: &[String], theirs: &[String]) -> Option<String> {
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
        "line {at}: ours {:?} vs in memory {:?}",
        ours.get(at).map(|s| short(s)),
        theirs.get(at).map(|s| short(s)),
    ))
}
