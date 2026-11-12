//! The append API (todo E2e) through both renderers: a page wrapped by
//! `laminapdf_content::PageContentWriter` renders exactly as before, and a square appended
//! after the wrap changes the pixels under it and nothing else, wherever the page's content left
//! its `q`/`Q` depth. hayro runs in every test run; PDFium's runs are ignored, as its other
//! tests are (`cargo xtask render-golden` runs them).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::path::Path;
use std::sync::Arc;

use common::{Bitmap, pdfium, render};
use laminapdf_content::PageContentWriter;
use laminapdf_model::{Document, Matrix, OpenOptions};
use laminapdf_render::{ByteSource, HayroBackend, RenderBackend, RenderFlags, RenderOpenOptions, RenderRequest};
use laminapdf_save::{FullWriter, IncrementalWriter, SaveOptions};
use laminapdf_test_fixtures::{fixture, fixtures_dir};

/// The append fixtures a wrap accepts (`unterminated.pdf` is refused).
const FIXTURES: [&str; 8] = [
    "balanced",
    "open-q",
    "extra-Q",
    "stray-Q",
    "split",
    "inline-image",
    "shared",
    "in-text",
];

/// The square every page gets, in pixels of the page rendered at 72 dpi.
const SQUARE: [u32; 4] = [20, 20, 60, 60];

/// The pages rendered per corpus file, to keep the run short.
const MAX_PAGES: usize = 3;

/// One edit of a page: given the writer and the matrix from the rendered page's pixels at
/// 72 dpi to user space, whether it changed the page.
type Edit = fn(&mut PageContentWriter<'_>, &Matrix) -> bool;

/// Opens `bytes`, edits the first [`MAX_PAGES`] pages with `edit` (a page it fails on is rolled
/// back), and saves incrementally, or as a full rewrite when the base can't take an update.
/// `None` when the file doesn't open or save, or no page was edited.
fn edited(bytes: Vec<u8>, edit: Edit) -> Option<Vec<u8>> {
    let mut doc = Document::open(bytes, OpenOptions::default()).ok()?;
    let mut any = false;
    for index in 0..doc.pages().ok()?.len().min(MAX_PAGES) {
        let (page, matrix) = {
            let pages = doc.pages().ok()?;
            let page = pages.get(index)?;
            // Page space counts `/UserUnit`; both renderers draw user space at 72 dpi without
            // it, so a rendered pixel is `user_unit` points of page space.
            let unit = page.user_unit().ok()?;
            let m = page.frame().ok()?.to_user_space_matrix();
            let scaled = Matrix {
                a: m.a * unit,
                b: m.b * unit,
                c: m.c * unit,
                d: m.d * unit,
                ..m
            };
            (page.cos().clone(), scaled)
        };
        let mark = doc.cos().journal_mark();
        let changed = match PageContentWriter::new(doc.cos_mut().edit(), &page) {
            Ok(mut writer) => edit(&mut writer, &matrix),
            Err(_) => false,
        };
        if changed {
            any = true;
        } else {
            doc.cos_mut().rollback_to(mark).ok()?;
        }
    }
    if !any {
        return None;
    }
    let mut out = Vec::new();
    if IncrementalWriter::new(doc.cos(), SaveOptions::default())
        .write(&mut out)
        .is_err()
    {
        out.clear();
        FullWriter::new(doc.cos(), SaveOptions::default())
            .write(&mut out)
            .ok()?;
    }
    Some(out)
}

fn wrap(writer: &mut PageContentWriter<'_>, _: &Matrix) -> bool {
    writer.wrap_existing().is_ok()
}

/// Wraps the page and appends [`SQUARE`] in opaque red, at its place in the rendered page.
fn wrap_and_square(writer: &mut PageContentWriter<'_>, m: &Matrix) -> bool {
    let [x0, y0, x1, y1] = SQUARE;
    let content = format!(
        "q {:.6} {:.6} {:.6} {:.6} {:.6} {:.6} cm 1 0 0 rg {x0} {y0} {} {} re f Q",
        m.a,
        m.b,
        m.c,
        m.d,
        m.e,
        m.f,
        x1 - x0,
        y1 - y0
    );
    writer.wrap_existing().is_ok() && writer.append(content.as_bytes()).is_ok()
}

/// Every page of `bytes` (up to [`MAX_PAGES`]) at 72 dpi, white, without annotations (drawn over
/// the content, they could cover the square). `None` when the file doesn't open or a page
/// doesn't render.
fn pages(backend: &dyn RenderBackend, bytes: Vec<u8>) -> Option<Vec<Bitmap>> {
    let source: Arc<dyn ByteSource> = Arc::new(bytes);
    let doc = backend.open(source, &RenderOpenOptions::new()).ok()?;
    let flags = RenderFlags {
        annotations: false,
        form_widgets: false,
        ..RenderFlags::default()
    };
    (0..doc.page_count().min(u32::try_from(MAX_PAGES).unwrap()))
        .map(|i| render(&*doc, &RenderRequest::page(i, 1.0).with_flags(flags)).ok())
        .collect()
}

fn append_fixture(name: &str) -> Vec<u8> {
    fixture(&format!("content/append/{name}.pdf"))
}

/// The fixtures with a `Q` at depth 0. hayro doesn't ignore such a `Q`, as PDFium and pdf.js
/// do: it saves a state of its own before the page's content (hayro-interpret 0.8,
/// `interpret::interpret`), so the stray `Q` restores the page's initial state, which the
/// blanking then changes. ISO 32000-2 §8.4.2 makes the `Q` an error either way.
const UNMATCHED_Q: [&str; 5] = ["extra-Q", "stray-Q", "split", "inline-image", "shared"];

fn wrapped_fixtures_render_unchanged(backend: &dyn RenderBackend, skip: &[&str]) {
    for name in FIXTURES.into_iter().filter(|n| !skip.contains(n)) {
        let before = pages(backend, append_fixture(name)).unwrap();
        let after = pages(backend, edited(append_fixture(name), wrap).unwrap()).unwrap();
        let pixels = |pages: &[Bitmap]| {
            pages
                .iter()
                .map(|p| (p.width, p.height, p.rgba.clone()))
                .collect::<Vec<_>>()
        };
        assert!(
            pixels(&before) == pixels(&after),
            "{name}: the wrapped page renders differently"
        );
    }
}

/// Whether `after` differs from `before` only under [`SQUARE`] (and its anti-aliased edge),
/// with the square's inside red. `Err` says where it doesn't.
fn only_the_square_changed(before: &Bitmap, after: &Bitmap) -> Result<(), String> {
    if (before.width, before.height) != (after.width, after.height) {
        return Err("the page size changed".into());
    }
    let [x0, y0, x1, y1] = SQUARE;
    for y in 0..after.height {
        for x in 0..after.width {
            let i = ((y * after.width + x) * 4) as usize;
            let (old, new) = (&before.rgba[i..i + 4], &after.rgba[i..i + 4]);
            let near = (x0.saturating_sub(1)..=x1).contains(&x) && (y0.saturating_sub(1)..=y1).contains(&y);
            let inside = (x0 + 1..x1 - 1).contains(&x) && (y0 + 1..y1 - 1).contains(&y);
            if inside && new != [255, 0, 0, 255] {
                return Err(format!("({x}, {y}) inside the square is {new:?}"));
            }
            if !near && old != new {
                return Err(format!("({x}, {y}) outside the square changed from {old:?} to {new:?}"));
            }
        }
    }
    Ok(())
}

/// Every file the corpus checks append to: the corpus, the goldens and the append fixtures.
fn corpus_files(skip: &std::collections::BTreeMap<String, String>) -> Vec<String> {
    let mut files = Vec::new();
    for dir in ["corpus", "golden", "golden/examples", "content/append"] {
        let mut names: Vec<String> = std::fs::read_dir(fixtures_dir().join(dir))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| Path::new(n).extension().is_some_and(|e| e == "pdf") && !skip.contains_key(n))
            .map(|n| format!("{dir}/{n}"))
            .collect();
        names.sort();
        files.extend(names);
    }
    files
}

fn a_square_appended_to_every_page_changes_only_its_pixels(backend: &dyn RenderBackend, skip_key: &str, skip: &[&str]) {
    let mut checked = Vec::new();
    let skipped = |file: &str| skip.iter().any(|name| file == format!("content/append/{name}.pdf"));
    for file in corpus_files(&common::skipped(skip_key))
        .into_iter()
        .filter(|f| !skipped(f))
    {
        let Some(bytes) = edited(fixture(&file), wrap_and_square) else {
            continue;
        };
        let Some(before) = pages(backend, fixture(&file)) else {
            continue;
        };
        qpdf_check(&file, &bytes);
        let after = pages(backend, bytes.clone()).unwrap_or_else(|| panic!("{file}: the edited file doesn't render"));
        assert_eq!(before.len(), after.len(), "{file}");
        for (index, (b, a)) in before.iter().zip(&after).enumerate() {
            if let Err(why) = only_the_square_changed(b, a) {
                panic!("{file} page {}: {why}", index + 1);
            }
        }
        checked.push(file);
    }
    assert!(checked.len() >= 30, "only {} files checked: {checked:?}", checked.len());
    eprintln!("{} files checked", checked.len());
}

/// `qpdf --check` on an edited file when qpdf is installed (the CI images that run the render
/// goldens don't all have it; `cargo xtask roundtrip` is where qpdf is required).
fn qpdf_check(file: &str, bytes: &[u8]) {
    if std::process::Command::new("qpdf").arg("--version").output().is_err() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("lamina-append-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file.replace('/', "_"));
    std::fs::write(&path, bytes).unwrap();
    let out = std::process::Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .unwrap();
    // Exit 0 is clean, 3 is warnings only (qpdf's own codes); 2 is an error.
    assert_ne!(
        out.status.code(),
        Some(2),
        "{file}: qpdf --check: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    std::fs::remove_file(&path).unwrap();
}

mod hayro {
    use super::*;

    #[test]
    fn wrapped_pages_render_unchanged() {
        wrapped_fixtures_render_unchanged(&HayroBackend::new(), &UNMATCHED_Q);
    }

    #[test]
    fn a_square_appended_to_every_page_changes_only_its_pixels() {
        super::a_square_appended_to_every_page_changes_only_its_pixels(&HayroBackend::new(), "hayro", &UNMATCHED_Q);
    }
}

mod pdfium {
    use super::*;

    #[test]
    #[ignore = "needs PDFium: cargo xtask render-golden"]
    fn wrapped_pages_render_unchanged() {
        wrapped_fixtures_render_unchanged(&pdfium(), &[]);
    }

    #[test]
    #[ignore = "needs PDFium: cargo xtask render-golden"]
    fn a_square_appended_to_every_page_changes_only_its_pixels() {
        super::a_square_appended_to_every_page_changes_only_its_pixels(&pdfium(), "pdfium", &[]);
    }
}
