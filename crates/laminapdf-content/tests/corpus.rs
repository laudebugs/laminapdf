//! ADR 0018 tier 4 for content: every page of `fixtures/corpus` (and `fixtures/golden`,
//! `fixtures/content`)
//! interprets, or fails with a coded error, and its operators match pdf.js's operator list
//! (`fixtures/corpus/pdfjs-operators.json`, written by `tools/corpus-facts/pdfjs-operators.mjs`)
//! once mapped as ADR 0023 §6 documents:
//!
//! * pdf.js folds path construction (`m l c v y h re`) into the `constructPath` of the
//!   painting operator that ends the path; `W`/`W*` are `clip`/`eoClip`.
//! * `CS`/`cs` emit nothing (the colour space is folded into the next colour), and every
//!   colour operator becomes `setStrokeRGBColor`/`setFillRGBColor`, except `SCN`/`scn` with a
//!   pattern (`setStrokeColorN`/`setFillColorN`).
//! * `MP`, `DP`, `BX` and `EX` emit nothing.
//! * `'` is `nextLine` + `showText`; `"` is `setWordSpacing` + `setCharSpacing` + `nextLine` +
//!   `showText`; `TJ` is `showText`.
//! * `gs` is `setGState` only when the dictionary has a parameter pdf.js renders (`LW LC LJ ML
//!   D RI FL CA ca Font BM SMask TR TR2`).
//! * Images: an image mask (`/ImageMask`, `/IM`) is `paintImageMaskXObject`; an inline image
//!   is `paintInlineImageXObject` only when `W + H < 200` and it has no `/SMask` or `/Mask`,
//!   else `paintImageXObject`; an image XObject with `/SMask` is wrapped in `save`,
//!   `setGState`, `restore`.
//! * A form with `/OC` is wrapped in `beginMarkedContentProps`/`endMarkedContent`, one with
//!   `/Group` in `beginGroup`/`endGroup` (inside the former).
//! * `dependency` operators (fonts and images loaded) are not content operators: the
//!   generator leaves them out. Annotations are excluded on both sides.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use std::collections::BTreeMap;

use laminapdf_content::{
    FormXObject, GraphicsStateParams, ImageDraw, ImageSource, Op, Operator, Visitor, interpret_page,
};
use laminapdf_cos::Object;
use laminapdf_model::{Document, OpenOptions};
use laminapdf_test_fixtures::fixtures_dir;
use serde_json::Value;

/// Counts Lamina PDF's operators under pdf.js's names.
#[derive(Default)]
struct PdfJsNames(BTreeMap<String, u64>);

impl PdfJsNames {
    fn add(&mut self, name: &str) {
        *self.0.entry(name.to_owned()).or_default() += 1;
    }
}

/// pdf.js's names for one operator (`Do` is counted by the image and form callbacks).
fn pdfjs_names(op: Operator<'_>) -> &'static [&'static str] {
    use Operator as O;
    match op {
        O::LineWidth => &["setLineWidth"],
        O::LineCap => &["setLineCap"],
        O::LineJoin => &["setLineJoin"],
        O::MiterLimit => &["setMiterLimit"],
        O::Dash => &["setDash"],
        O::RenderingIntent => &["setRenderingIntent"],
        O::Flatness => &["setFlatness"],
        O::Save => &["save"],
        O::Restore => &["restore"],
        O::Transform => &["transform"],
        O::Stroke
        | O::CloseStroke
        | O::Fill
        | O::FillObsolete
        | O::FillEvenOdd
        | O::FillStroke
        | O::FillStrokeEvenOdd
        | O::CloseFillStroke
        | O::CloseFillStrokeEvenOdd
        | O::EndPath => &["constructPath"],
        O::Clip => &["clip"],
        O::ClipEvenOdd => &["eoClip"],
        O::BeginText => &["beginText"],
        O::EndText => &["endText"],
        O::CharSpacing => &["setCharSpacing"],
        O::WordSpacing => &["setWordSpacing"],
        O::HorizontalScaling => &["setHScale"],
        O::Leading => &["setLeading"],
        O::Font => &["setFont"],
        O::RenderMode => &["setTextRenderingMode"],
        O::Rise => &["setTextRise"],
        O::MoveText => &["moveText"],
        O::MoveTextSetLeading => &["setLeadingMoveText"],
        O::TextMatrix => &["setTextMatrix"],
        O::NextLine => &["nextLine"],
        O::ShowText | O::ShowTextAdjusted => &["showText"],
        O::NextLineShowText => &["nextLine", "showText"],
        O::SpacingNextLineShowText => &["setWordSpacing", "setCharSpacing", "nextLine", "showText"],
        O::GlyphWidth => &["setCharWidth"],
        O::GlyphWidthBBox => &["setCharWidthAndBounds"],
        O::StrokeColor | O::StrokeColorN | O::StrokeGray | O::StrokeRgb | O::StrokeCmyk => &["setStrokeRGBColor"],
        O::FillColor | O::FillColorN | O::FillGray | O::FillRgb | O::FillCmyk => &["setFillRGBColor"],
        O::Shading => &["shadingFill"],
        O::BeginMarked => &["beginMarkedContent"],
        O::BeginMarkedProperties => &["beginMarkedContentProps"],
        O::EndMarked => &["endMarkedContent"],
        // Inline images and `Do` are counted by the image and form callbacks; pdf.js keeps no
        // operator for colour spaces, marked-content points or compatibility sections.
        _ => &[],
    }
}

impl Visitor for PdfJsNames {
    fn operator(&mut self, op: &Op<'_>, _: usize) {
        let pattern = matches!(op.operands.last(), Some(Object::Name(_)));
        match op.operator {
            Operator::StrokeColorN if pattern => return self.add("setStrokeColorN"),
            Operator::FillColorN if pattern => return self.add("setFillColorN"),
            _ => {}
        }
        for name in pdfjs_names(op.operator) {
            self.add(name);
        }
    }

    fn set_graphics_state(&mut self, params: &GraphicsStateParams<'_>) {
        // pdf.js keeps an ExtGState only for the parameters it renders.
        const RENDERED: [&str; 14] = [
            "LW", "LC", "LJ", "ML", "D", "RI", "FL", "CA", "ca", "Font", "BM", "SMask", "TR", "TR2",
        ];
        if RENDERED.iter().any(|key| params.dict.contains_key(key)) {
            self.add("setGState");
        }
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        match image.source {
            ImageSource::Inline(inline) => {
                let get = |short: &str, long: &str| inline.dict.get(short).or_else(|| inline.dict.get(long));
                let dimension = |short, long| get(short, long).and_then(Object::as_int).unwrap_or(0);
                let masked = get("SMask", "SMask").is_some() || get("Mask", "Mask").is_some();
                if matches!(get("IM", "ImageMask"), Some(Object::Bool(true))) {
                    self.add("paintImageMaskXObject");
                } else if !masked && dimension("W", "Width") + dimension("H", "Height") < 200 {
                    self.add("paintInlineImageXObject");
                } else {
                    self.add("paintImageXObject");
                }
            }
            ImageSource::XObject { stream, .. } => {
                let dict = stream.as_dict().unwrap();
                if matches!(dict.get("ImageMask"), Some(Object::Bool(true))) {
                    self.add("paintImageMaskXObject");
                } else if dict.contains_key("SMask") {
                    for name in ["save", "setGState", "paintImageXObject", "restore"] {
                        self.add(name);
                    }
                } else {
                    self.add("paintImageXObject");
                }
            }
            _ => {}
        }
    }

    fn enter_xobject(&mut self, form: &FormXObject<'_>) {
        if form.dict.contains_key("OC") {
            self.add("beginMarkedContentProps");
        }
        if form.dict.contains_key("Group") {
            self.add("beginGroup");
        }
        self.add("paintFormXObjectBegin");
    }

    fn leave_xobject(&mut self, form: &FormXObject<'_>) {
        self.add("paintFormXObjectEnd");
        if form.dict.contains_key("Group") {
            self.add("endGroup");
        }
        if form.dict.contains_key("OC") {
            self.add("endMarkedContent");
        }
    }
}

fn expected() -> serde_json::Map<String, Value> {
    let bytes = std::fs::read(fixtures_dir().join("corpus/pdfjs-operators.json")).unwrap();
    let Value::Object(map) = serde_json::from_slice(&bytes).unwrap() else {
        panic!("an object")
    };
    map
}

/// Every page's counts under pdf.js's names, or the coded error that stopped it.
fn ours(file: &str) -> Result<Vec<BTreeMap<String, u64>>, &'static str> {
    let bytes = std::fs::read(fixtures_dir().join("..").join(file)).unwrap();
    let doc = Document::open(bytes, OpenOptions::default()).map_err(|e| e.code())?;
    let pages = doc.pages().map_err(|e| e.code())?;
    pages
        .iter()
        .map(|page| {
            let mut names = PdfJsNames::default();
            interpret_page(&doc, &page, &mut names).map_err(|e| e.code())?;
            Ok(names.0)
        })
        .collect()
}

#[test]
fn every_fixture_is_listed_in_the_pdfjs_facts() {
    let expected = expected();
    for dir in ["corpus", "golden", "content"] {
        for entry in std::fs::read_dir(fixtures_dir().join(dir)).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if std::path::Path::new(&name).extension().is_some_and(|x| x == "pdf") {
                let key = format!("fixtures/{dir}/{name}");
                assert!(expected.contains_key(&key), "{key} missing from pdfjs-operators.json");
            }
        }
    }
}

#[test]
fn every_page_interprets_or_fails_with_an_excused_error() {
    let excused = exceptions();
    for file in expected().keys() {
        if let Err(code) = ours(file) {
            assert!(
                excused.contains(&(file.clone(), code.to_owned())),
                "{file}: {code} is not in pdfjs-exceptions.toml"
            );
        }
    }
}

/// `(file, error code)` pairs `pdfjs-exceptions.toml` excuses, each with its ADR 0003 rule.
fn exceptions() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(fixtures_dir().join("corpus/pdfjs-exceptions.toml")).unwrap();
    let table: toml::Table = text.parse().unwrap();
    // None at the moment: every fixture interprets.
    table
        .get("exception")
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|e| {
            assert!(e["rule"].as_str().unwrap().starts_with("ADR 0003"), "{e:?}");
            (
                e["file"].as_str().unwrap().to_owned(),
                e["code"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn operator_counts_match_pdfjs() {
    let excused = exceptions();
    let mut failures = Vec::new();
    for (file, theirs) in expected() {
        let Value::Array(theirs) = theirs else {
            failures.push(format!("{file}: pdf.js failed: {theirs}"));
            continue;
        };
        let theirs: Vec<BTreeMap<String, u64>> = theirs
            .iter()
            .map(|page| serde_json::from_value(page.clone()).unwrap())
            .collect();
        match ours(&file) {
            Ok(ours) if ours == theirs => {}
            Ok(ours) => failures.push(format!("{file}:\n  ours   {ours:?}\n  pdf.js {theirs:?}")),
            Err(code) if excused.contains(&(file.clone(), code.to_owned())) => {}
            Err(code) => failures.push(format!("{file}: {code}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_operators_fixture_interprets_without_a_warning() {
    let bytes = std::fs::read(fixtures_dir().join("content/operators.pdf")).unwrap();
    let doc = Document::open(bytes, OpenOptions::default().with_strict(true)).unwrap();
    let pages = doc.pages().unwrap();
    let summary = interpret_page(&doc, &pages.get(0).unwrap(), &mut PdfJsNames::default()).unwrap();
    assert_eq!(summary.forms, 4);
}
