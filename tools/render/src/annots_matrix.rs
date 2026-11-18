//! The appearance parity matrix: one PDF holding every annotation subtype Lamina PDF draws
//! (ISO 32000-2 §12.5.6), under each [`LOOKS`] property set (colour space and colour × opacity ×
//! border), on pages turned 0°, 90°, 180° and 270°. Page `look × 4 + turn` holds look `look` on
//! turn `turn`. Besides the subtypes, the matrix covers each variant that draws differently: a
//! text box, a callout and typewriter text; a plain line and a dimension line with leader lines;
//! a caret with and without its paragraph symbol (alternate pages); and every note icon
//! ([`ICONS`], one per page in turn). After the matrix comes `fixtures/forms/text-layout.pdf`'s
//! page with every field filled ([`FORM_VALUES`]): the form field appearances of todo E8d, one
//! per layout rule, copied in with their fields.
//!
//! Everything goes through the façade as an app would: the builder draws the words the text
//! marks cover, the bytes are opened, the marks are made with the façade's `*_at` methods and
//! applied, and the file is saved incrementally. Pages are square, so page space (the page as
//! shown) has the same layout at every turn: a renderer that turns an appearance the wrong way
//! shows it in the wrong place. The bytes depend on nothing but this code (fixed file ID, no
//! dates), so the same build writes the same file.

use anyhow::{Context, Result};
use laminapdf::{
    AddAnnot, AnnotColor, FieldId, FieldValue, InsertPagesFrom, PagePosition, SetFieldValue, AnnotSpec, Border, BuildOptions, Builder, CaretSymbol, Document, FreeTextIntent, Leader,
    LineEnding, MarkupKind, NoteIcon, Op, Opacity, OpenOptions, PageSize, PointPt, PolyKind, Pt, RectPt, RichText,
    Rotation, SaveMode, SaveOptions, ShapeKind, Standard14,
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
}

/// Every note icon (ISO 32000-2 Table 175); page `p` shows `ICONS[p % 7]`, so the 16 pages show
/// each at least twice, on different turns.
pub const ICONS: [NoteIcon; 7] = [
    NoteIcon::Comment,
    NoteIcon::Note,
    NoteIcon::Help,
    NoteIcon::Key,
    NoteIcon::Paragraph,
    NoteIcon::NewParagraph,
    NoteIcon::Insert,
];

/// The property sets: the three colour spaces `/C` can name (RGB, CMYK, gray), opaque and half
/// transparent, and every border style.
pub const LOOKS: [Look; 4] = [
    Look {
        name: "red, opaque, solid 1 pt",
        color: AnnotColor::Rgb(0.85, 0.1, 0.1),
        opacity: 1.0,
        border: || Border::solid(1.0),
        plain: || Border::solid(1.0),
        interior: None,
        ends: [LineEnding::None, LineEnding::ClosedArrow],
    },
    Look {
        name: "blue, 50 %, dashed 2 pt",
        color: AnnotColor::Rgb(0.1, 0.3, 0.9),
        opacity: 0.5,
        border: || Border::dashed(2.0, vec![4.0, 2.0]),
        plain: || Border::dashed(2.0, vec![4.0, 2.0]),
        interior: Some(AnnotColor::Rgb(1.0, 0.9, 0.3)),
        ends: [LineEnding::Circle, LineEnding::OpenArrow],
    },
    Look {
        name: "CMYK green, opaque, cloudy",
        color: AnnotColor::Cmyk(0.85, 0.1, 0.95, 0.1),
        opacity: 1.0,
        border: || Border::cloudy(1.0, 1.0),
        plain: || Border::solid(1.0),
        interior: None,
        ends: [LineEnding::Square, LineEnding::Diamond],
    },
    Look {
        name: "grey, 50 %, solid 4 pt",
        color: AnnotColor::Gray(0.35),
        opacity: 0.5,
        border: || Border::solid(4.0),
        plain: || Border::solid(4.0),
        interior: Some(AnnotColor::Gray(0.8)),
        ends: [LineEnding::Butt, LineEnding::RClosedArrow],
    },
];

/// The subtypes, in the order each page lists them (the grid row by row); each cell holds one.
pub const SUBTYPES: [&str; 16] = [
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
    "FreeText",
    "Line",
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
    let form = doc.add_source(filled_form()?, None)?;
    let pages = doc.source_pages(&form)?;
    doc.apply(vec![Op::from(InsertPagesFrom::new(form, pages, PagePosition::End))])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// What each field of `text-layout.pdf` is filled with; `latin` takes Noto Sans as its fallback.
pub const FORM_VALUES: [(&str, &str); 12] = [
    ("auto", "Auto-sized to the box"),
    ("multi", "A longer answer that wraps over several lines and shrinks until every line fits the box."),
    ("comb", "AB12CD34"),
    ("left", "Left"),
    ("center", "Centred"),
    ("right", "Right"),
    ("r90", "Turned 90"),
    ("r180", "Turned 180"),
    ("r270", "Turned 270"),
    ("secret", "hunter2"),
    ("nohelv", "Helvetica, which /DR lacks"),
    ("latin", "Łódź, Gdańsk"),
];

/// `text-layout.pdf` with every field filled, its combo box and list box among them.
fn filled_form() -> Result<Vec<u8>> {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let bytes = std::fs::read(fixtures.join("forms/text-layout.pdf")).context("reading text-layout.pdf")?;
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let noto = doc.add_blob(std::fs::read(fixtures.join("fonts/NotoSans-Regular.ttf")).context("reading Noto Sans")?);
    let fields = doc.form()?.fields;
    let id = |name: &str| -> Result<FieldId> {
        let field = fields.iter().find(|f| f.name == name).with_context(|| format!("no field {name}"))?;
        field.id.parse().map_err(|_| anyhow::anyhow!("field {name}'s ID"))
    };
    let mut ops: Vec<Op> = Vec::new();
    for (name, value) in FORM_VALUES {
        let fonts = if name == "latin" { vec![noto] } else { Vec::new() };
        ops.push(SetFieldValue::new(id(name)?, FieldValue::Text(value.into())).with_fonts(fonts).into());
    }
    ops.push(SetFieldValue::new(id("pick")?, FieldValue::Choice(vec!["Banana".into()])).into());
    ops.push(SetFieldValue::new(id("rows")?, FieldValue::Choice(vec!["Two".into(), "Four".into()])).into());
    ops.push(SetFieldValue::new(id("tick")?, FieldValue::Button(Some("Yes".into()))).into());
    doc.apply(ops)?;
    Ok(doc.save(&SaveOptions::new().with_mode(SaveMode::Incremental))?.bytes)
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
    let icon = ICONS
        .get(usize::try_from(page)? % ICONS.len())
        .copied()
        .unwrap_or_default();
    marks.push(doc.note_at(page, point(x + 35.0, y + 30.0), "A note", icon)?);
    let [x, y] = cell_origin(5);
    let mut caret = doc.caret_at(page, rect(x + 35.0, y + 35.0, x + 55.0, y + 60.0))?;
    if let AnnotSpec::Caret(spec) = &mut caret.spec
        && page % 2 == 1
    {
        spec.symbol = CaretSymbol::Paragraph;
    }
    marks.push(caret);
    marks.push(ink(doc, page, look)?);
    marks.extend(shapes(doc, page, look)?);
    marks.extend(lines(doc, page, look)?);
    marks.extend(text_boxes(doc, page, look)?);
    marks.push(dimension_line(doc, page, look)?);
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
    let [x, y] = cell_origin(14);
    let mut typewriter = doc.text_box_at(
        page,
        rect(x + 10.0, y + 35.0, x + 90.0, y + 65.0),
        RichText::plain("Typed"),
    )?;
    if let AnnotSpec::FreeText(spec) = &mut typewriter.spec {
        spec.intent = FreeTextIntent::Typewriter;
    }
    for mark in [&mut text_box, &mut callout] {
        if let AnnotSpec::FreeText(spec) = &mut mark.spec {
            spec.border = (look.border)();
            spec.fill = look.interior;
            if let FreeTextIntent::Callout(callout) = &mut spec.intent {
                callout.end = look.ends[1];
            }
        }
    }
    Ok(vec![text_box, callout, typewriter])
}

/// A dimension line: leader lines from both ends set it off from what it measures.
fn dimension_line(doc: &Document, page: u32, look: &Look) -> Result<AddAnnot> {
    let [x, y] = cell_origin(15);
    let mut dimension = doc.line_at(page, point(x + 15.0, y + 60.0), point(x + 85.0, y + 60.0), look.ends)?;
    if let AnnotSpec::Line(spec) = &mut dimension.spec {
        let mut leader = Leader::new(20.0);
        leader.extension = 5.0;
        spec.leader = Some(leader);
        spec.border = (look.plain)();
        spec.interior = look.interior;
    }
    Ok(dimension)
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

    fn matrix() -> Document {
        Document::open_bytes(generate().unwrap(), OpenOptions::new()).unwrap()
    }

    #[test]
    fn the_same_build_writes_the_same_bytes() {
        assert_eq!(generate().unwrap(), generate().unwrap());
    }

    #[test]
    fn there_is_a_page_for_every_look_on_every_turn() {
        assert_eq!(matrix().page_count().unwrap(), u32::try_from(PAGES).unwrap());
    }

    #[test]
    fn every_page_holds_every_subtype() {
        let doc = matrix();
        for page in 0..doc.page_count().unwrap() {
            let subtypes: Vec<String> = doc
                .page_annotations(page)
                .unwrap()
                .into_iter()
                .filter(|a| a.subtype != "Popup")
                .map(|a| a.subtype)
                .collect();
            assert_eq!(subtypes, SUBTYPES, "page {page}");
        }
    }

    #[test]
    fn the_pages_of_each_look_take_every_turn_in_order() {
        let turns: Vec<u16> = matrix().page_infos().unwrap().iter().map(|p| p.rotation).collect();
        let expected: Vec<u16> = (0..LOOKS.len()).flat_map(|_| TURNS.map(Rotation::degrees)).collect();
        assert_eq!(turns, expected);
    }

    #[test]
    fn every_mark_has_an_appearance() {
        let annots = matrix().page_annotations(0).unwrap();
        assert!(annots.iter().filter(|a| a.subtype != "Popup").all(|a| a.has_appearance));
    }
}
