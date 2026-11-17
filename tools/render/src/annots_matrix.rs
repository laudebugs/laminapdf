//! The appearance parity matrix: one PDF holding every annotation subtype Lamina PDF draws
//! (ISO 32000-2 §12.5.6), under each [`LOOKS`] property set (colour × opacity × border), on pages
//! turned 0°, 90°, 180° and 270°. Page `look × 4 + turn` holds look `look` on turn `turn`.
//!
//! Everything goes through the façade as an app would: the builder draws the words the text
//! marks cover, the bytes are opened, the marks are made with the façade's `*_at` methods and
//! applied, and the file is saved incrementally. Pages are square, so page space (the page as
//! shown) has the same layout at every turn: a renderer that turns an appearance the wrong way
//! shows it in the wrong place. The bytes depend on nothing but this code (fixed file ID, no
//! dates), so the same build writes the same file.

use anyhow::{Context, Result};
use laminapdf::{
    AddAnnot, AnnotColor, AnnotSpec, Border, BuildOptions, Builder, Document, FreeTextIntent, LineEnding, MarkupKind,
    NoteIcon, Op, Opacity, OpenOptions, PageSize, PointPt, PolyKind, Pt, RectPt, RichText, Rotation, SaveMode,
    SaveOptions, ShapeKind, Standard14,
};

/// Side of the square page, in points.
const PAGE: f64 = 400.0;
/// Side of one cell of the 4 × 4 grid each page is divided into.
const CELL: f64 = 100.0;
/// The word under each text mark, and its size.
const WORD: &str = "Lamina";
const WORD_SIZE: f64 = 14.0;
/// `WORD`'s advance in Helvetica at `WORD_SIZE`: L a m i n a = 556 + 556 + 833 + 222 + 556 + 556
/// thousandths of an em (the standard 14 fonts' AFM widths).
const WORD_WIDTH: f64 = 3.279 * WORD_SIZE;

/// The turns of the pages, in page order within each look.
pub const TURNS: [Rotation; 4] = [
    Rotation::None,
    Rotation::Clockwise90,
    Rotation::Half,
    Rotation::Clockwise270,
];

/// One property set: what every mark on a page shares.
pub struct Look {
    /// For reports: what the set varies.
    pub name: &'static str,
    color: AnnotColor,
    opacity: f64,
    /// The border of the subtypes that can be cloudy (squares, circles, polygons, text boxes).
    border: fn() -> Border,
    /// The border of the others (lines and polylines, which have no cloudy style).
    plain: fn() -> Border,
    /// The fill of shapes and closed line endings.
    interior: Option<AnnotColor>,
    /// The endings of lines and polylines, and the callout's (its end only).
    ends: [LineEnding; 2],
    icon: NoteIcon,
}

/// The property sets: each colour once, opaque and half transparent, and every border style.
pub const LOOKS: [Look; 4] = [
    Look {
        name: "red, opaque, solid 1 pt",
        color: AnnotColor::Rgb(0.85, 0.1, 0.1),
        opacity: 1.0,
        border: || Border::solid(1.0),
        plain: || Border::solid(1.0),
        interior: None,
        ends: [LineEnding::None, LineEnding::ClosedArrow],
        icon: NoteIcon::Comment,
    },
    Look {
        name: "blue, 50 %, dashed 2 pt",
        color: AnnotColor::Rgb(0.1, 0.3, 0.9),
        opacity: 0.5,
        border: || Border::dashed(2.0, vec![4.0, 2.0]),
        plain: || Border::dashed(2.0, vec![4.0, 2.0]),
        interior: Some(AnnotColor::Rgb(1.0, 0.9, 0.3)),
        ends: [LineEnding::Circle, LineEnding::OpenArrow],
        icon: NoteIcon::Note,
    },
    Look {
        name: "green, opaque, cloudy",
        color: AnnotColor::Rgb(0.1, 0.6, 0.2),
        opacity: 1.0,
        border: || Border::cloudy(1.0, 1.0),
        plain: || Border::solid(1.0),
        interior: None,
        ends: [LineEnding::Square, LineEnding::Diamond],
        icon: NoteIcon::Help,
    },
    Look {
        name: "grey, 50 %, solid 4 pt",
        color: AnnotColor::Gray(0.35),
        opacity: 0.5,
        border: || Border::solid(4.0),
        plain: || Border::solid(4.0),
        interior: Some(AnnotColor::Gray(0.8)),
        ends: [LineEnding::Butt, LineEnding::RClosedArrow],
        icon: NoteIcon::Key,
    },
];

/// The subtypes, in grid order (row by row); each cell holds one.
pub const SUBTYPES: [&str; 14] = [
    "Highlight",
    "Underline",
    "StrikeOut",
    "Squiggly",
    "Text",
    "Caret",
    "Ink",
    "Square",
    "Circle",
    "Line",
    "Polygon",
    "PolyLine",
    "FreeText",
    "FreeText",
];

/// The text marks, in their cells (the first four).
const MARKUPS: [MarkupKind; 4] = [
    MarkupKind::Highlight,
    MarkupKind::Underline,
    MarkupKind::StrikeOut,
    MarkupKind::Squiggly,
];

/// The page count: every look on every turn.
pub const PAGES: usize = LOOKS.len() * TURNS.len();

/// The matrix PDF.
pub fn generate() -> Result<Vec<u8>> {
    let mut doc = Document::open_bytes(pages()?, OpenOptions::new())?;
    for (l, look) in LOOKS.iter().enumerate() {
        for t in 0..TURNS.len() {
            let page = u32::try_from(l * TURNS.len() + t)?;
            let ops = marks(&doc, page, look)?.into_iter().map(Op::from).collect();
            doc.apply(ops)?;
        }
    }
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// The pages before any mark: the word in each text mark's cell.
fn pages() -> Result<Vec<u8>> {
    let mut b = Builder::new(BuildOptions::new().with_id(*b"annots-parity-v1"));
    let helvetica = b.standard_font(Standard14::Helvetica)?;
    for _ in &LOOKS {
        for turn in TURNS {
            let mut page = b.page(PageSize::new(Pt::new(PAGE), Pt::new(PAGE)));
            page.rotate(turn);
            for cell in (0..).take(MARKUPS.len()) {
                let [x, y] = word_origin(cell);
                page.surface()
                    .text(WORD, &helvetica, Pt::new(WORD_SIZE), PointPt::from_points(x, y))?;
            }
            page.finish()?;
        }
    }
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// Every mark of `look` on `page`, one per cell.
fn marks(doc: &Document, page: u32, look: &Look) -> Result<Vec<AddAnnot>> {
    let mut marks = Vec::new();
    for (cell, kind) in (0..).zip(MARKUPS) {
        let [x, y] = word_origin(cell);
        // The word's line box: its ascent (0.72 em, Helvetica's cap height) above the baseline,
        // its descent (0.22 em) below.
        let line = rect(x, y - 0.72 * WORD_SIZE, x + WORD_WIDTH, y + 0.22 * WORD_SIZE);
        marks.push(doc.mark_text(page, kind, &[line])?);
    }
    let [x, y] = cell_origin(4);
    marks.push(doc.note_at(page, point(x + 35.0, y + 30.0), "A note", look.icon)?);
    let [x, y] = cell_origin(5);
    marks.push(doc.caret_at(page, rect(x + 35.0, y + 35.0, x + 55.0, y + 60.0))?);
    marks.push(ink(doc, page, look)?);
    marks.extend(shapes(doc, page, look)?);
    marks.extend(lines(doc, page, look)?);
    marks.extend(text_boxes(doc, page, look)?);
    for mark in &mut marks {
        mark.common.color = Some(look.color);
        mark.common.opacity = Opacity::new(look.opacity).context("an opacity from 0 to 1")?;
    }
    Ok(marks)
}

fn ink(doc: &Document, page: u32, look: &Look) -> Result<AddAnnot> {
    let [x, y] = cell_origin(6);
    let zigzag = |y0: f64| -> Vec<PointPt> {
        (0..6)
            .map(|i| point(x + 15.0 + 14.0 * f64::from(i), y0 + if i % 2 == 0 { 0.0 } else { 18.0 }))
            .collect()
    };
    Ok(doc.ink_at(page, &[zigzag(y + 20.0), zigzag(y + 55.0)], (look.plain)().width)?)
}

fn shapes(doc: &Document, page: u32, look: &Look) -> Result<Vec<AddAnnot>> {
    let mut marks = Vec::new();
    for (cell, kind) in [(7, ShapeKind::Square), (8, ShapeKind::Circle)] {
        let [x, y] = cell_origin(cell);
        let mut mark = doc.shape_at(page, kind, rect(x + 15.0, y + 25.0, x + 85.0, y + 75.0))?;
        if let AnnotSpec::Shape(shape) = &mut mark.spec {
            shape.border = (look.border)();
            shape.interior = look.interior;
        }
        marks.push(mark);
    }
    Ok(marks)
}

fn lines(doc: &Document, page: u32, look: &Look) -> Result<Vec<AddAnnot>> {
    let [x, y] = cell_origin(9);
    let mut line = doc.line_at(page, point(x + 15.0, y + 80.0), point(x + 85.0, y + 20.0), look.ends)?;
    if let AnnotSpec::Line(spec) = &mut line.spec {
        spec.border = (look.plain)();
        spec.interior = look.interior;
    }
    let [x, y] = cell_origin(10);
    let pentagon = [(50.0, 15.0), (85.0, 42.0), (72.0, 85.0), (28.0, 85.0), (15.0, 42.0)];
    let vertices: Vec<PointPt> = pentagon.iter().map(|&(dx, dy)| point(x + dx, y + dy)).collect();
    let mut polygon = doc.poly_at(page, PolyKind::Polygon, &vertices)?;
    if let AnnotSpec::Poly(spec) = &mut polygon.spec {
        spec.border = (look.border)();
        spec.interior = look.interior;
    }
    let [x, y] = cell_origin(11);
    let steps = [(15.0, 80.0), (35.0, 30.0), (55.0, 70.0), (85.0, 20.0)];
    let vertices: Vec<PointPt> = steps.iter().map(|&(dx, dy)| point(x + dx, y + dy)).collect();
    let mut polyline = doc.poly_at(page, PolyKind::PolyLine, &vertices)?;
    if let AnnotSpec::Poly(spec) = &mut polyline.spec {
        spec.border = (look.plain)();
        spec.interior = look.interior;
        spec.ends = look.ends;
    }
    Ok(vec![line, polygon, polyline])
}

fn text_boxes(doc: &Document, page: u32, look: &Look) -> Result<Vec<AddAnnot>> {
    let [x, y] = cell_origin(12);
    let mut text_box = doc.text_box_at(
        page,
        rect(x + 10.0, y + 25.0, x + 90.0, y + 75.0),
        RichText::plain("Check the totals"),
    )?;
    let [x, y] = cell_origin(13);
    let leader = [
        point(x + 10.0, y + 85.0),
        point(x + 30.0, y + 60.0),
        point(x + 40.0, y + 60.0),
    ];
    let mut callout = doc.callout_at(
        page,
        rect(x + 40.0, y + 15.0, x + 95.0, y + 60.0),
        &leader,
        RichText::plain("Why?"),
    )?;
    for mark in [&mut text_box, &mut callout] {
        if let AnnotSpec::FreeText(spec) = &mut mark.spec {
            spec.border = (look.border)();
            spec.fill = look.interior;
            if let FreeTextIntent::Callout(callout) = &mut spec.intent {
                callout.end = look.ends[1];
            }
        }
    }
    Ok(vec![text_box, callout])
}

/// The top-left corner of cell `cell` (row by row) in page space.
fn cell_origin(cell: u8) -> [f64; 2] {
    [CELL * f64::from(cell % 4), CELL * f64::from(cell / 4)]
}

/// Where the word of text mark `cell` starts: its baseline, left.
fn word_origin(cell: u8) -> [f64; 2] {
    let [x, y] = cell_origin(cell);
    [x + (CELL - WORD_WIDTH) / 2.0, y + 55.0]
}

fn point(x: f64, y: f64) -> PointPt {
    PointPt::new(Pt::new(x), Pt::new(y))
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_build_writes_the_same_bytes() {
        assert_eq!(generate().unwrap(), generate().unwrap());
    }

    #[test]
    fn every_page_holds_every_subtype_on_its_turn() {
        let doc = Document::open_bytes(generate().unwrap(), OpenOptions::new()).unwrap();
        assert_eq!(doc.page_count().unwrap(), u32::try_from(PAGES).unwrap());
        for page in 0..PAGES {
            let index = u32::try_from(page).unwrap();
            let subtypes: Vec<String> = doc
                .page_annotations(index)
                .unwrap()
                .into_iter()
                .filter(|a| a.subtype != "Popup")
                .map(|a| a.subtype)
                .collect();
            assert_eq!(subtypes, SUBTYPES, "page {page}");
            assert_eq!(
                doc.page_info(index).unwrap().rotation,
                TURNS[page % TURNS.len()].degrees()
            );
        }
    }

    #[test]
    fn every_mark_has_an_appearance() {
        let doc = Document::open_bytes(generate().unwrap(), OpenOptions::new()).unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert!(annots.iter().filter(|a| a.subtype != "Popup").all(|a| a.has_appearance));
    }
}
