//! Shared helpers for lamina-content's integration tests: small documents built around a
//! content stream, and a visitor that records every event.
#![allow(
    dead_code,
    unreachable_pub,
    reason = "each test crate uses a different subset of the helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

pub mod fuzz_entry;

use std::path::{Path, PathBuf};

use lamina_content::{
    FillRule, FormXObject, ImageDraw, ImageSource, Matrix, Op, Paint, PathSegment, TextAdvance, TextItem, TextShow,
    Visitor, Warning, interpret_content, interpret_page,
};
use lamina_cos::{Dict, ObjRef, Object, Stream, Writer};
use lamina_model::{Document, OpenOptions, Result};

pub use lamina_content::Summary;

/// The repository's shared `fixtures/` directory.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Every `corpus/*.pdf` as `(file name, bytes)`, sorted by name.
pub fn corpus() -> Vec<(String, Vec<u8>)> {
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
    files
}

/// A dictionary from `(key, value)` pairs.
pub fn dict<const N: usize>(entries: [(&str, Object); N]) -> Dict {
    entries.into_iter().collect()
}

/// A one-page document. `build` adds objects with the writer and returns the page content and
/// its resource dictionary.
pub fn page_doc(build: impl FnOnce(&mut Writer) -> (Vec<u8>, Dict)) -> Document {
    page_doc_with(OpenOptions::default(), build)
}

/// [`page_doc`] with `options`.
pub fn page_doc_with(options: OpenOptions, build: impl FnOnce(&mut Writer) -> (Vec<u8>, Dict)) -> Document {
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let (content, resources) = build(&mut w);
    let content = w.add(Stream::new(Dict::new(), content));
    let page = w.add(dict([
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ("Contents", content.into()),
        ("Resources", resources.into()),
    ]));
    w.set(
        pages,
        dict([
            ("Type", Object::name("Pages")),
            ("Kids", vec![page.into()].into()),
            ("Count", 1.into()),
        ]),
    );
    let root = w.add(dict([("Type", Object::name("Catalog")), ("Pages", pages.into())]));
    let bytes = w.finish(&[("Root", root)].into_iter().collect()).unwrap();
    Document::open(bytes, options).unwrap()
}

/// A document with `content` on its page and a Type 1 font `/F1`.
pub fn text_doc(content: &[u8]) -> Document {
    page_doc(|w| (content.to_vec(), font_resources(w)))
}

/// Resources with one Helvetica font named `F1`.
pub fn font_resources(w: &mut Writer) -> Dict {
    let font = w.add(dict([
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
    ]));
    dict([("Font", dict([("F1", font.into())]).into())])
}

/// A Form XObject stream with `content` and `extra` dictionary entries.
pub fn form(content: &[u8], mut extra: Dict) -> Stream {
    for (key, value) in [
        ("Type", Object::name("XObject")),
        ("Subtype", Object::name("Form")),
        ("BBox", vec![0.into(), 0.into(), 10.into(), 10.into()].into()),
    ] {
        if !extra.contains_key(key) {
            extra.insert(key, value);
        }
    }
    Stream::new(extra, content.to_vec())
}

/// Interprets the first page of `doc`, recording events.
pub fn run(doc: &Document) -> (Result<Summary>, Recorder) {
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let mut rec = Recorder::default();
    let result = interpret_page(doc, &page, &mut rec);
    (result, rec)
}

/// Interprets `content` with no resources in an empty document.
pub fn run_content(content: &[u8]) -> (Result<Summary>, Recorder) {
    let doc = text_doc(b"");
    let mut rec = Recorder::default();
    let result = interpret_content(&doc, content, None, &mut rec);
    (result, rec)
}

/// Interprets the first page of `doc` with `visitor`.
pub fn run_with<V: Visitor>(doc: &Document, visitor: &mut V) -> Result<Summary> {
    let pages = doc.pages().unwrap();
    interpret_page(doc, &pages.get(0).unwrap(), visitor)
}

/// A recorded `paint_path`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaintPathEvent {
    pub paint: Paint,
    pub clip: Option<FillRule>,
}

/// One recorded event, simplified for assertions.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Show {
        strings: Vec<Vec<u8>>,
        tm: Matrix,
        ctm: Matrix,
        font: Option<Vec<u8>>,
        font_size: f64,
        depth: usize,
    },
    Segment(PathSegment),
    Paint(PaintPathEvent),
    Shading(Vec<u8>),
    Image {
        inline: bool,
        ctm: Matrix,
    },
    Begin {
        tag: Vec<u8>,
        mcid: Option<i64>,
        depth: usize,
    },
    End,
    Point(Vec<u8>),
    Enter {
        object: Option<ObjRef>,
        depth: usize,
        ctm: Matrix,
    },
    Leave(Option<ObjRef>),
}

/// Records every event; text shows advance by `advance_per_byte` text space units per byte.
#[derive(Debug, Default)]
pub struct Recorder {
    pub events: Vec<Event>,
    pub warnings: Vec<Warning>,
    pub operators: Vec<(Vec<u8>, usize)>,
    pub advance_per_byte: f64,
}

impl Recorder {
    pub fn codes(&self) -> Vec<&'static str> {
        self.warnings.iter().map(Warning::code).collect()
    }

    pub fn shows(&self) -> Vec<&Event> {
        self.events.iter().filter(|e| matches!(e, Event::Show { .. })).collect()
    }
}

impl Visitor for Recorder {
    fn operator(&mut self, op: &Op<'_>, depth: usize) {
        self.operators.push((op.operator.keyword().to_vec(), depth));
    }

    fn show_text(&mut self, show: &TextShow<'_>) -> TextAdvance {
        let strings: Vec<Vec<u8>> = show
            .items()
            .filter_map(|i| match i {
                TextItem::Text(t) => Some(t.to_vec()),
                TextItem::Adjust(_) => None,
            })
            .collect();
        let bytes: usize = strings.iter().map(Vec::len).sum();
        self.events.push(Event::Show {
            strings,
            tm: show.tm,
            ctm: show.state.ctm,
            font: show.font().and_then(|f| f.name.as_ref()).map(|n| n.as_bytes().to_vec()),
            font_size: show.state.text.font_size,
            depth: show.depth,
        });
        #[expect(clippy::cast_precision_loss, reason = "test strings are short")]
        let tx = bytes as f64 * self.advance_per_byte;
        TextAdvance { tx, ty: 0.0 }
    }

    fn path_segment(&mut self, segment: &PathSegment, _: &Matrix) {
        self.events.push(Event::Segment(*segment));
    }

    fn paint_path(&mut self, paint: &lamina_content::PathPaint<'_>) {
        self.events.push(Event::Paint(PaintPathEvent {
            paint: paint.paint,
            clip: paint.clip,
        }));
    }

    fn paint_shading(&mut self, shading: &lamina_content::ShadingPaint<'_>) {
        self.events.push(Event::Shading(shading.name.as_bytes().to_vec()));
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        self.events.push(Event::Image {
            inline: matches!(image.source, ImageSource::Inline(_)),
            ctm: image.state.ctm,
        });
    }

    fn begin_marked(&mut self, marked: &lamina_content::MarkedContent<'_>) {
        self.events.push(Event::Begin {
            tag: marked.tag.as_bytes().to_vec(),
            mcid: marked.mcid,
            depth: marked.depth,
        });
    }

    fn end_marked(&mut self) {
        self.events.push(Event::End);
    }

    fn marked_point(&mut self, marked: &lamina_content::MarkedContent<'_>) {
        self.events.push(Event::Point(marked.tag.as_bytes().to_vec()));
    }

    fn enter_xobject(&mut self, form: &FormXObject<'_>) {
        self.events.push(Event::Enter {
            object: form.object,
            depth: form.depth,
            ctm: form.state.ctm,
        });
    }

    fn leave_xobject(&mut self, form: &FormXObject<'_>) {
        self.events.push(Event::Leave(form.object));
    }

    fn warning(&mut self, warning: &Warning) {
        self.warnings.push(*warning);
    }
}
