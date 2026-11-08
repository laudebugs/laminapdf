//! The document behind `fixtures/golden/tagged-across-pages.pdf` (todo T4f): three pages under
//! PDF/UA-1 where a paragraph runs from page 1 to page 2 and a table from page 2 to page 3, each
//! one element ([`Surface::open_element`], [`Surface::resume`], [`Builder::close_element`]).
//! The paragraph holds a link on page 2; the table's rows are elements of their own pages inside
//! it. Every page has a running footer: a pagination artifact with the footer subtype, attached to
//! the bottom edge.

use laminapdf_build::{
    Artifact, BuildOptions, Builder, ElementId, FontHandle, LinkTarget, Metadata, PageSize, Profile, Result, Surface,
    TableScope, Tag, TagKind,
};
use laminapdf_model::{PointPt, Pt, RectPt};

use super::{save, text_and_navigation::font};

/// The paragraph's lines, page by page.
pub const PARAGRAPH: [&[&str]; 2] = [
    &[
        "This paragraph starts at the foot of the first page",
        "and the page break falls in the middle of it,",
    ],
    &["so it ends at the top of the second page."],
];
/// The link's `/Contents`, inside the paragraph on page 2.
pub const LINK: &str = "The full figures on example.com";
/// The table's rows, page by page; the first row is its header.
pub const ROWS: [&[[&str; 2]]; 2] = [
    &[["Quarter", "Sales"], ["Q1", "1,200"]],
    &[["Q2", "3,100"], ["Q3", "2,800"]],
];

const SIZE: f64 = 11.0;

fn at(x: f64, y: f64) -> PointPt {
    PointPt::from_points(x, y)
}

fn line(s: &mut Surface, text: &str, font: &FontHandle, (x, y): (f64, f64)) -> Result<()> {
    s.text(text, font, Pt::new(SIZE), at(x, y)).map(|_| ())
}

fn footer(s: &mut Surface, font: &FontHandle, page: usize) -> Result<()> {
    s.artifact(Artifact::footer(), |s| {
        line(s, &format!("Page {page} of 3"), font, (72.0, 800.0))
    })
}

/// Rows of the table drawn on one page, from `y` down, each a `TR` of that page.
fn rows(s: &mut Surface, font: &FontHandle, rows: &[[&str; 2]], header: bool, y: f64) -> Result<()> {
    for (i, row) in rows.iter().enumerate() {
        let y = y + 20.0 * f64::from(u8::try_from(i).unwrap_or(u8::MAX));
        s.tag(TagKind::TR, |s| {
            for (j, cell) in row.iter().enumerate() {
                let tag = if header && i == 0 {
                    Tag::new(TagKind::TH).with_scope(TableScope::Column)
                } else {
                    Tag::new(TagKind::TD)
                };
                let x = 72.0 + 150.0 * f64::from(u8::try_from(j).unwrap_or(u8::MAX));
                s.tag(tag, |s| line(s, cell, font, (x, y)))?;
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn first_page(s: &mut Surface, font: &FontHandle) -> Result<ElementId> {
    s.tag(TagKind::H1, |s| {
        s.text("Sales across pages", font, Pt::new(24.0), at(72.0, 96.0))
            .map(|_| ())
    })?;
    let paragraph = s.open_element(TagKind::P);
    s.resume(paragraph, |s| {
        for (i, text) in PARAGRAPH[0].iter().enumerate() {
            line(
                s,
                text,
                font,
                (72.0, 740.0 + 16.0 * f64::from(u8::try_from(i).unwrap_or(0))),
            )?;
        }
        Ok(())
    })?;
    footer(s, font, 1)?;
    Ok(paragraph)
}

fn second_page(s: &mut Surface, font: &FontHandle, paragraph: ElementId) -> Result<ElementId> {
    s.resume(paragraph, |s| {
        line(s, PARAGRAPH[1][0], font, (72.0, 72.0))?;
        s.link(
            RectPt::from_xywh(Pt::new(72.0), Pt::new(76.0), Pt::new(200.0), Pt::new(16.0)),
            LinkTarget::Uri("https://example.com/figures".into()),
            Some(LINK.into()),
        );
        Ok(())
    })?;
    let table = s.open_element(TagKind::Table);
    s.resume(table, |s| rows(s, font, ROWS[0], true, 720.0))?;
    footer(s, font, 2)?;
    Ok(table)
}

fn third_page(s: &mut Surface, font: &FontHandle, table: ElementId) -> Result<()> {
    s.resume(table, |s| rows(s, font, ROWS[1], false, 72.0))?;
    s.tag(TagKind::P, |s| line(s, "The table ends here.", font, (72.0, 140.0)))?;
    footer(s, font, 3)
}

/// The builder with every page written and both elements closed.
pub fn across_pages_builder() -> Builder {
    let mut b = Builder::new(
        BuildOptions::default()
            .with_id(*b"lamina-build-T4f")
            .with_producer("lamina-build golden")
            .with_creation_date("D:20261108120000Z".parse().unwrap())
            .with_compress(false)
            .with_profile(Profile::PdfUa1),
    );
    let noto = b.font(font("NotoSans-Regular.ttf")).unwrap();
    let mut page = b.page(PageSize::A4);
    let paragraph = first_page(page.surface(), &noto).unwrap();
    page.finish().unwrap();
    let mut page = b.page(PageSize::A4);
    let table = second_page(page.surface(), &noto, paragraph).unwrap();
    page.finish().unwrap();
    b.close_element(paragraph).unwrap();
    let mut page = b.page(PageSize::A4);
    third_page(page.surface(), &noto, table).unwrap();
    page.finish().unwrap();
    b.close_element(table).unwrap();
    b.metadata(Metadata::default().with_title("Sales across pages"))
        .lang("en-GB");
    b
}

/// `fixtures/golden/tagged-across-pages.pdf`.
pub fn tagged_across_pages() -> Vec<u8> {
    save(across_pages_builder().finish().unwrap())
}
