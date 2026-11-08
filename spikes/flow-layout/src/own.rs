//! Experiment A: a minimal paginator of our own over `laminapdf-build`'s `Surface`. It flows
//! headings, paragraphs and tables down A4 pages, keeps a heading with two lines of what
//! follows, leaves no single line at the foot of a page, repeats a table's header row on every
//! page it crosses, tags everything, and draws a running header and a "Page X of Y" footer as
//! pagination artifacts. The page count comes from a first pass, because a finished page can't be
//! reopened and the builder has no content to fill in later.

use laminapdf::{
    Alignment, ArtifactKind, BuildOptions, Builder, Color, Document, Font, FontHandle, Metadata, PageSize, Profile,
    Pt, RectPt, StrokeStyle, Surface, Tag, TableScope, TagKind, TextBoxOptions,
};

use crate::inputs::{Block, Doc};

type Result<T> = std::result::Result<T, laminapdf::Error>;
/// What the surface's own calls (and their closures) return.
type Drawn<T> = std::result::Result<T, laminapdf::BuildError>;

const PAGE: PageSize = PageSize::A4;
const MARGIN_X: f64 = 56.0;
const TOP: f64 = 72.0;
const BOTTOM: f64 = PAGE.height.get() - 64.0;
const WIDTH: f64 = PAGE.width.get() - 2.0 * MARGIN_X;
const BODY: f64 = 11.0;
const CELL: f64 = 10.0;
/// The text box's default line height, as a factor of the size.
const LEADING: f64 = 1.2;
const GAP: f64 = 6.0;
const ROW: f64 = CELL * LEADING + 4.0;

struct Fonts {
    regular: FontHandle,
    bold: FontHandle,
    arabic: FontHandle,
    devanagari: FontHandle,
}

/// What the paginator learnt about the builder, recorded for the spike's README.
#[derive(Debug, Default, Clone)]
pub struct Gaps {
    /// Paragraphs split across a page break: each part became its own `P` element.
    pub split_paragraphs: usize,
    /// Tables split across a page break: each part became its own `Table` element.
    pub split_tables: usize,
    /// Every page drawn twice: once to count the pages, once with "of Y".
    pub passes: usize,
}

/// Lays `doc` out twice (the page count, then the pages) and returns the document; `ua` checks
/// PDF/UA-1 as the builder writes it.
pub fn build(doc: &Doc, fonts: &[Vec<u8>], ua: bool) -> Result<(Document, usize, Gaps)> {
    let (_, pages, _) = pass(doc, fonts, None, ua)?;
    let (document, again, mut gaps) = pass(doc, fonts, Some(pages), ua)?;
    debug_assert_eq!(pages, again, "the footer's text doesn't move the flow");
    gaps.passes = 2;
    Ok((document, pages, gaps))
}

fn pass(doc: &Doc, fonts: &[Vec<u8>], total: Option<usize>, ua: bool) -> Result<(Document, usize, Gaps)> {
    let profile = if ua { Profile::PdfUa1 } else { Profile::None };
    let options = BuildOptions::new().with_id(*b"lamina-flow-spk!").with_tagged(true).with_profile(profile);
    let mut b = Builder::new(options);
    b.lang("en");
    b.metadata(Metadata::default().with_title(doc.title.clone()));
    let f = Fonts {
        regular: b.font(Font::from_bytes(fonts[0].clone())?)?,
        bold: b.font(Font::from_collection(fonts[1].clone(), 1)?)?,
        arabic: b.font(Font::from_bytes(fonts[2].clone())?)?,
        devanagari: b.font(Font::from_bytes(fonts[3].clone())?)?,
    };
    let mut gaps = Gaps::default();
    // How much height two body lines take: a text box's first baseline sits one ascent below its
    // top, so this is more than two line heights. Measured once, in a throwaway builder, because
    // nothing in the builder measures text without drawing it.
    let two_lines = two_lines(&fonts[0])?;
    let mut number = 1;
    let mut page = b.page(PAGE);
    furniture(page.surface(), &f, &doc.title, number, total)?;
    let mut y = TOP;
    // Starts a new page: the old one is finished, its furniture already drawn.
    macro_rules! break_page {
        () => {{
            page.finish()?;
            number += 1;
            page = b.page(PAGE);
            furniture(page.surface(), &f, &doc.title, number, total)?;
            y = TOP;
        }};
    }
    for (i, block) in doc.blocks.iter().enumerate() {
        match block {
            Block::Heading(level, text) => {
                let size = if *level == 1 { 16.0 } else { 13.0 };
                // Keep with next: the heading, a gap and two lines of the next block.
                let next = match doc.blocks.get(i + 1) {
                    Some(Block::Table { .. }) => 2.0 * ROW,
                    _ => two_lines,
                };
                // One line's box (ascent to descent) scales with the size: two lines less a line height.
                let heading = (two_lines - BODY * LEADING) / BODY * size;
                if y + heading + GAP + next > BOTTOM {
                    break_page!();
                }
                let kind = if *level == 1 { TagKind::H1 } else { TagKind::H2 };
                let s = page.surface();
                let laid = s.tag(kind, |s| s.text_box(text, &f.bold, Pt::new(size), area(y), &TextBoxOptions::new()))?;
                y = laid.bottom().map_or(y, |b| b.get()) + GAP;
            }
            Block::Para(text) | Block::Script { text, .. } => {
                let (font, options) = match block {
                    Block::Script { lang: "ar", .. } => (&f.arabic, TextBoxOptions::new().with_language("ar")),
                    Block::Script { .. } => (&f.devanagari, TextBoxOptions::new().with_language("hi")),
                    _ => (&f.regular, TextBoxOptions::new()),
                };
                let mut rest = text.as_str();
                loop {
                    // No orphan: a paragraph never starts on the last line of a page.
                    if y + two_lines > BOTTOM {
                        break_page!();
                    }
                    let s = page.surface();
                    let laid = s.tag(TagKind::P, |s| s.text_box(rest, font, Pt::new(BODY), area(y), &options))?;
                    match laid.overflow {
                        None => {
                            y = laid.bottom().map_or(y, |b| b.get()) + GAP;
                            break;
                        }
                        Some(cut) => {
                            gaps.split_paragraphs += 1;
                            rest = rest[cut..].trim_start();
                            break_page!();
                        }
                    }
                }
            }
            Block::Table { header, rows } => {
                let mut next = 0;
                while next < rows.len() {
                    if y + 2.0 * ROW > BOTTOM {
                        break_page!();
                    }
                    let fit = (((BOTTOM - y) / ROW).floor() as usize).saturating_sub(1).max(1);
                    let end = (next + fit).min(rows.len());
                    let s = page.surface();
                    y = s.tag(TagKind::Table, |s| table_part(s, &f, header, &rows[next..end], y))?;
                    if end < rows.len() {
                        gaps.split_tables += 1;
                        break_page!();
                    }
                    next = end;
                }
                y += GAP;
            }
        }
    }
    page.finish()?;
    Ok((b.finish()?, number, gaps))
}

/// The height a text box needs for two body lines, from the top of its first line to the
/// bottom of its second.
fn two_lines(regular: &[u8]) -> Result<f64> {
    let mut b = Builder::new(BuildOptions::new());
    let font = b.font(Font::from_bytes(regular.to_vec())?)?;
    let mut page = b.page(PAGE);
    let laid = page.surface().text_box("A\nA", &font, Pt::new(BODY), area(TOP), &TextBoxOptions::new())?;
    Ok(laid.bottom().map_or(2.0 * BODY * LEADING, |b| b.get() - TOP))
}

/// The rest of the page's text column, from `y` down to the bottom margin.
fn area(y: f64) -> RectPt {
    RectPt::from_points(MARGIN_X, y, WIDTH, (BOTTOM - y).max(0.0))
}

/// The header row, then `rows`, from `y`; returns where the part ends.
fn table_part(s: &mut Surface, f: &Fonts, header: &[String], rows: &[Vec<String>], y: f64) -> Drawn<f64> {
    // Fixed weights: the first column gets 40 %. The builder can't measure a cell before drawing
    // it, so columns can't be sized to their content, and a cell too narrow clips silently.
    let rest = (WIDTH * 0.6) / (header.len().saturating_sub(1).max(1)) as f64;
    let left = |c: usize| if c == 0 { 0.0 } else { WIDTH * 0.4 + (c - 1) as f64 * rest };
    let width = |c: usize| if c == 0 { WIDTH * 0.4 } else { rest };
    let mut y = y;
    let row = |s: &mut Surface, cells: &[String], font: &FontHandle, kind: TagKind, y: f64| -> Drawn<()> {
        s.tag(TagKind::TR, |s| {
            for (c, cell) in cells.iter().enumerate() {
                let x = MARGIN_X + left(c) + 2.0;
                let rect = RectPt::from_points(x, y + 2.0, width(c) - 4.0, ROW);
                let tag = if kind == TagKind::TH { Tag::new(kind).with_scope(TableScope::Column) } else { Tag::new(kind) };
                s.tag(tag, |s| s.text_box(cell, font, Pt::new(CELL), rect, &TextBoxOptions::new()))?;
            }
            Ok(())
        })
    };
    row(s, header, &f.bold, TagKind::TH, y)?;
    y += ROW;
    s.artifact(ArtifactKind::Layout, |s| {
        let line = RectPt::from_points(MARGIN_X, y, WIDTH, 0.0);
        s.stroke_color(Color::gray(0.5)).path(|p| _ = p.rect(line)).stroke(&StrokeStyle::default());
        Ok(())
    })?;
    for cells in rows {
        row(s, cells, &f.regular, TagKind::TD, y)?;
        y += ROW;
    }
    Ok(y)
}

/// The running header and the "Page X of Y" footer, as pagination artifacts.
fn furniture(s: &mut Surface, f: &Fonts, title: &str, number: usize, total: Option<usize>) -> Drawn<()> {
    s.artifact(ArtifactKind::Pagination, |s| {
        let header = RectPt::from_points(MARGIN_X, 36.0, WIDTH, 14.0);
        let right = TextBoxOptions::new().with_align(Alignment::End);
        s.text_box(title, &f.regular, Pt::new(9.0), header, &right)?;
        let footer = RectPt::from_points(MARGIN_X, PAGE.height.get() - 40.0, WIDTH, 14.0);
        let text = match total {
            Some(total) => format!("Page {number} of {total}"),
            None => format!("Page {number} of {number}"),
        };
        s.text_box(&text, &f.regular, Pt::new(9.0), footer, &TextBoxOptions::new().with_align(Alignment::Center))?;
        Ok(())
    })
}
