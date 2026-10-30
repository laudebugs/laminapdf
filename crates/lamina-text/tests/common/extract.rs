//! One-page documents for the extraction tests (todos B9b, B9d): content, fonts and page
//! entries given by the test, written with `lamina-save`'s spike-era `Writer`.
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers: a panic is a test failure"
)]

use lamina_cos::OpenOptions;
use lamina_model::Document;
use lamina_save::Writer;
use lamina_syntax::{Dict, Object, Stream};
use lamina_text::{PageText, TextExtractor};

/// A one-page document with `content` and the fonts `fonts` as `/F1`, `/F2`, ….
pub fn page(content: &[u8], fonts: Vec<Dict>, options: OpenOptions) -> Document {
    page_with(content, fonts, Vec::new(), options)
}

/// [`page`] with more page entries (`/Rotate`, `/UserUnit`, `/CropBox`, …).
pub fn page_with(content: &[u8], fonts: Vec<Dict>, entries: Vec<(&str, Object)>, options: OpenOptions) -> Document {
    let dict = |entries: Vec<(&str, Object)>| entries.into_iter().collect::<Dict>();
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let font_refs: Dict = fonts
        .into_iter()
        .enumerate()
        .map(|(i, font)| {
            (
                lamina_syntax::Name::new(format!("F{}", i + 1)),
                Object::from(w.add(font)),
            )
        })
        .collect();
    let content = w.add(Stream::new(Dict::new(), content.to_vec()));
    let mut page = dict(vec![
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ("Contents", content.into()),
        ("Resources", dict(vec![("Font", font_refs.into())]).into()),
    ]);
    for (key, value) in entries {
        page.insert(key, value);
    }
    let page = w.add(page);
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
    Document::open(bytes, options).expect("opens")
}

pub fn font(source: &str) -> Dict {
    match lamina_syntax::parse_object(source.as_bytes()) {
        Ok(Object::Dict(d)) => d,
        other => panic!("{other:?}"),
    }
}

pub fn helvetica() -> Dict {
    font("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
}

pub fn extract(doc: &Document) -> lamina_cos::Result<PageText> {
    let pages = doc.pages()?;
    let page = pages.get(0).expect("a page");
    TextExtractor::new().page(doc, &page)
}
