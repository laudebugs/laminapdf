//! `Builder::on`: pages added to an open document through its overlay, saved incrementally,
//! with every existing page keeping its ID (ADR 0005 §6).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, record, reopen};
use lamina_build::{Builder, Color, FillRule, PageSize, Rotation};
use lamina_cos::{Dict, Object};
use lamina_model::{PageId, Pt, RectPt};
use lamina_save::{IncrementalWriter, SaveOptions, Writer};

fn ids(doc: &lamina_model::Document) -> Vec<String> {
    doc.pages().unwrap().iter().map(|p| p.id().to_string()).collect()
}

/// Adds one Letter page with a filled square to `base`, saved incrementally.
fn append_square(base: &[u8]) -> (Vec<u8>, Vec<PageId>) {
    let mut doc = reopen(base);
    let mut appender = Builder::on(&mut doc).unwrap();
    let mut page = appender.page(PageSize::LETTER);
    page.surface()
        .fill_color(Color::rgb(0.1, 0.5, 0.2))
        .path(|p| {
            p.rect(RectPt::from_xywh(
                Pt::new(72.0),
                Pt::new(72.0),
                Pt::new(100.0),
                Pt::new(100.0),
            ));
        })
        .fill(FillRule::NonZero);
    page.finish().unwrap();
    let added = appender.finish().unwrap();
    let mut out = Vec::new();
    IncrementalWriter::new(doc.cos(), SaveOptions::default())
        .write(&mut out)
        .unwrap();
    (out, added)
}

#[test]
fn a_page_added_to_hello_saves_incrementally_and_keeps_the_existing_ids() {
    let base = fixture("corpus/hello.pdf");
    let before = ids(&reopen(&base));
    let (out, added) = append_square(&base);
    let after = reopen(&out);
    let mut expected = before;
    expected.extend(added.iter().map(ToString::to_string));
    assert_eq!(
        (out.starts_with(&base), ids(&after), after.pages().unwrap().len()),
        (true, expected, 2)
    );
}

#[test]
fn the_added_page_draws_its_content_and_reads_without_warnings() {
    let (out, _) = append_square(&fixture("corpus/hello.pdf"));
    let doc = reopen(&out);
    let drawn = record(&doc, 1);
    assert_eq!(
        (drawn.painted[0].points.clone(), drawn.warnings, doc.warnings().len()),
        (vec![(72.0, 620.0), (172.0, 720.0)], vec![], 0)
    );
}

/// A document whose root `/Pages` has an inherited `/Rotate 90` and `/CropBox`, and whose
/// `/Kids` is an indirect array.
fn inheriting_base() -> Vec<u8> {
    let dict = |entries: Vec<(&str, Object)>| -> Dict { entries.into_iter().collect() };
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let kids = w.reserve();
    let page = w.add(dict(vec![("Type", Object::name("Page")), ("Parent", pages.into())]));
    w.set(kids, Object::Array(vec![page.into()]));
    let media: Object = vec![0.into(), 0.into(), 612.into(), 792.into()].into();
    let crop: Object = vec![0.into(), 0.into(), 300.into(), 300.into()].into();
    w.set(
        pages,
        dict(vec![
            ("Type", Object::name("Pages")),
            ("Kids", kids.into()),
            ("Count", 1.into()),
            ("MediaBox", media),
            ("CropBox", crop),
            ("Rotate", 90.into()),
        ]),
    );
    let root = w.add(dict(vec![("Type", Object::name("Catalog")), ("Pages", pages.into())]));
    w.finish(&[("Root", Object::from(root))].into_iter().collect()).unwrap()
}

#[test]
fn an_added_page_inherits_nothing_from_the_tree_root() {
    let (out, _) = append_square(&inheriting_base());
    let doc = reopen(&out);
    let pages = doc.pages().unwrap();
    let (old, new) = (pages.get(0).unwrap(), pages.get(1).unwrap());
    assert_eq!(
        (
            old.rotate().unwrap(),
            new.rotate().unwrap(),
            new.crop_box().unwrap().map(|r| r.to_array()),
            pages.len()
        ),
        (Rotation::Clockwise90, Rotation::None, Some([0.0, 0.0, 612.0, 792.0]), 2)
    );
}

#[test]
fn a_catalog_without_an_indirect_page_tree_is_refused() {
    let mut w = Writer::new(1, 7);
    let pages: Object = Object::Dict(
        [
            ("Type", Object::name("Pages")),
            ("Kids", Object::Array(vec![])),
            ("Count", 0.into()),
        ]
        .into_iter()
        .collect(),
    );
    let root = w.add(
        [("Type", Object::name("Catalog")), ("Pages", pages)]
            .into_iter()
            .collect::<Dict>(),
    );
    let bytes = w.finish(&[("Root", Object::from(root))].into_iter().collect()).unwrap();
    let mut doc = reopen(&bytes);
    assert_eq!(Builder::on(&mut doc).unwrap_err().code(), "invalid-structure");
}
