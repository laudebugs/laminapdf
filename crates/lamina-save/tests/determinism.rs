//! Byte-deterministic output (ADR 0001): every writer, run twice on the same input, produces
//! identical bytes. Each run builds its input from scratch, so nothing is shared between runs.

mod common;

use common::fixture;
use common::update;
use lamina_cos::{Dict, ObjRef, Object, PdfString, Stream};
use lamina_save::{Writer, XrefEol};

/// A small document using every object type, with forward references and a stream.
fn build() -> (Writer, Dict) {
    let mut w = Writer::new(1, 7);
    let catalog = w.reserve();
    let pages = w.reserve();
    let content = Stream::flate(Dict::new(), b"BT /F1 12 Tf 72 720 Td (Deterministic) Tj ET");
    let content = w.add(content);
    let font: Dict = [
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
    ]
    .into_iter()
    .collect();
    let font = w.add(font);
    let fonts: Dict = [("F1", Object::from(font))].into_iter().collect();
    let resources: Dict = [("Font", Object::Dict(fonts))].into_iter().collect();
    let page: Dict = [
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.5.into()].into()),
        ("Resources", resources.into()),
        ("Contents", content.into()),
    ]
    .into_iter()
    .collect();
    let page = w.add(page);
    let pages_dict: Dict = [
        ("Type", Object::name("Pages")),
        ("Kids", vec![page.into()].into()),
        ("Count", 1.into()),
    ]
    .into_iter()
    .collect();
    w.set(pages, pages_dict);
    let catalog_dict: Dict = [("Type", Object::name("Catalog")), ("Pages", pages.into())]
        .into_iter()
        .collect();
    w.set(catalog, catalog_dict);
    let info: Dict = [
        ("Title", Object::from(PdfString::literal("Twice (the same)"))),
        ("Trapped", Object::Bool(false)),
        ("Extra", Object::Null),
    ]
    .into_iter()
    .collect();
    let info = w.add(info);
    let trailer: Dict = [("Root", catalog), ("Info", info)].into_iter().collect();
    (w, trailer)
}

#[test]
fn classic_writer_should_produce_identical_bytes_when_run_twice() {
    let run = || {
        let (w, trailer) = build();
        w.finish(&trailer).unwrap()
    };
    assert_eq!(run(), run());
}

#[test]
fn compact_writer_should_produce_identical_bytes_when_run_twice() {
    let run = || {
        let (w, trailer) = build();
        w.finish_compact(&trailer).unwrap()
    };
    assert_eq!(run(), run());
}

#[test]
fn incremental_update_should_produce_identical_bytes_when_run_twice() {
    let run = || {
        let base = fixture("golden/hello.pdf");
        let changes = [
            (ObjRef::new(5, 0), Stream::new(Dict::new(), b"BT ET".to_vec()).into()),
            (ObjRef::new(6, 0), Object::from(PdfString::hex(b"new".to_vec()))),
            (ObjRef::new(4, 0), Object::name("Replaced")),
        ];
        update(&base, &changes, XrefEol::SpaceLf).unwrap()
    };
    assert_eq!(run(), run());
}

#[test]
fn incremental_update_of_compact_file_should_produce_identical_bytes_when_run_twice() {
    let run = || {
        let (w, trailer) = build();
        let base = w.finish_compact(&trailer).unwrap();
        update(&base, &[(ObjRef::new(3, 0), Object::Null)], XrefEol::CrLf).unwrap()
    };
    assert_eq!(run(), run());
}

#[test]
fn golden_samples_should_produce_identical_bytes_when_run_twice() {
    assert_eq!(common::legacy::hello().unwrap(), common::legacy::hello().unwrap());
    assert_eq!(
        common::legacy::imaging_demo().unwrap(),
        common::legacy::imaging_demo().unwrap()
    );
}
