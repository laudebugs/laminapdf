//! The document behind `fixtures/golden/tagged-report.pdf` (todo B8a): a two-page tagged
//! report with headings, paragraphs, a figure with alternate text and its caption, a table
//! with column headers, a list, links inside `Link` elements (a text run's and a standalone
//! annotation's), a `Span` with its own `/Lang`, a custom type mapped by the `RoleMap`, and
//! artifacts (running heads and footers, a rule, a page background). [`STRUCTURE`] is the tree
//! the calls build, which tests compare with what lamina-model and pdf.js read back.

use lamina_build::{
    ArtifactKind, BuildOptions, Builder, Color, FillRule, Font, Image, LinkTarget, Metadata, PageSize, StrokeStyle,
    Surface, TableScope, Tag, TagKind, TextBoxOptions,
};
use lamina_model::{PointPt, Pt, RectPt};

use super::{b5b::font, save};

/// The figure's alternate text.
pub const FIGURE_ALT: &str = "A bar chart of sales by quarter, rising from 120 to 310";
/// The text run link's `/Contents`.
pub const RUN_LINK: &str = "The full report on example.com";
/// The standalone link's `/Contents`.
pub const PAGE_LINK: &str = "Back to the first page";

const HEADERS: [&str; 3] = ["Quarter", "Units", "Sales"];
const ROWS: [[&str; 3]; 2] = [["Q1", "120", "€ 1,200"], ["Q2", "310", "€ 3,100"]];
const ITEMS: [&str; 2] = ["Widgets sold out in May.", "Gadgets ship in the autumn."];

/// The structure the report builds, in reading order: each element's depth below the
/// `Document` element, its `/S` and its `/Alt`.
pub const STRUCTURE: &[(usize, &str, Option<&str>)] = &[
    (0, "H1", None),
    (0, "P", None),
    (0, "Figure", Some(FIGURE_ALT)),
    (0, "Caption", None),
    (0, "Table", None),
    (1, "TR", None),
    (2, "TH", None),
    (2, "TH", None),
    (2, "TH", None),
    (1, "TR", None),
    (2, "TD", None),
    (2, "TD", None),
    (2, "TD", None),
    (1, "TR", None),
    (2, "TD", None),
    (2, "TD", None),
    (2, "TD", None),
    (0, "L", None),
    (1, "LI", None),
    (2, "Lbl", None),
    (2, "LBody", None),
    (1, "LI", None),
    (2, "Lbl", None),
    (2, "LBody", None),
    (0, "P", None),
    (1, "Link", None),
    (0, "P", None),
    (1, "Span", None),
    (0, "Total", None),
    (0, "Sect", None),
    (1, "H2", None),
    (1, "P", None),
    (1, "Quote", None),
    (1, "Note", None),
    (1, "Code", None),
    (0, "Link", None),
];

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

fn at(x: f64, y: f64) -> PointPt {
    PointPt::from_points(x, y)
}

type Res = lamina_build::Result<()>;

/// A line of text in the surface's current element.
fn line(s: &mut Surface, text: &str, font: &Font, size: f64, x: f64, y: f64) -> Res {
    s.text(text, font, Pt::new(size), at(x, y)).map(|_| ())
}

fn tagged(s: &mut Surface, tag: impl Into<Tag>, text: &str, font: &Font, size: f64, x: f64, y: f64) -> Res {
    s.tag(tag, |s| line(s, text, font, size, x, y))
}

/// The running head and footer of page `n` (pagination artifacts).
fn pagination(s: &mut Surface, font: &Font, n: u32) -> Res {
    s.artifact(ArtifactKind::Pagination, |s| {
        line(s, "Acme Widgets · Quarterly report", font, 9.0, 56.0, 40.0)?;
        line(s, &format!("Page {n}"), font, 9.0, 520.0, 810.0)
    })
}

fn first_page(s: &mut Surface, noto: &Font) -> Res {
    pagination(s, noto, 1)?;
    tagged(s, TagKind::H1, "Quarterly report", noto, 24.0, 56.0, 96.0)?;
    s.tag(TagKind::P, |s| {
        s.text_box(
            "Sales rose in every region this quarter. The chart and the table below show units and \
             revenue per quarter.",
            noto,
            Pt::new(11.0),
            rect(56.0, 110.0, 483.0, 40.0),
            &TextBoxOptions::default(),
        )
        .map(|_| ())
    })?;
    let chart = Image::rgba(
        4,
        1,
        &[
            30, 120, 200, 255, 30, 120, 200, 255, 30, 120, 200, 255, 30, 120, 200, 255,
        ],
    )?;
    s.figure(&chart, rect(56.0, 160.0, 200.0, 80.0), Some(FIGURE_ALT));
    tagged(
        s,
        TagKind::Caption,
        "Figure 1: sales by quarter",
        noto,
        9.0,
        56.0,
        254.0,
    )?;
    s.artifact(ArtifactKind::Layout, |s| {
        s.stroke_color(Color::gray(0.6))
            .path(|p| {
                p.move_to(at(56.0, 266.0)).line_to(at(539.0, 266.0));
            })
            .stroke(&StrokeStyle::default());
        Ok(())
    })?;
    s.tag(TagKind::Table, |s| {
        let header = Tag::new(TagKind::TH).with_scope(TableScope::Column);
        s.tag(TagKind::TR, |s| {
            for (i, h) in (0_u32..).zip(HEADERS) {
                tagged(s, header.clone(), h, noto, 11.0, 56.0 + 120.0 * f64::from(i), 290.0)?;
            }
            Ok(())
        })?;
        for (r, row) in (0_u32..).zip(ROWS) {
            s.tag(TagKind::TR, |s| {
                for (i, cell) in (0_u32..).zip(row) {
                    let y = 310.0 + 18.0 * f64::from(r);
                    tagged(s, TagKind::TD, cell, noto, 11.0, 56.0 + 120.0 * f64::from(i), y)?;
                }
                Ok(())
            })?;
        }
        Ok(())
    })?;
    s.tag(TagKind::L, |s| {
        for (i, item) in (0_u32..).zip(ITEMS) {
            let y = 370.0 + 18.0 * f64::from(i);
            s.tag(TagKind::LI, |s| {
                tagged(s, TagKind::Lbl, "•", noto, 11.0, 56.0, y)?;
                tagged(s, TagKind::LBody, item, noto, 11.0, 70.0, y)
            })?;
        }
        Ok(())
    })?;
    let _ = s
        .text(
            "Read the full report at example.com/report",
            noto,
            Pt::new(11.0),
            at(56.0, 420.0),
        )?
        .fill(Color::rgb(0.12, 0.47, 0.78))
        .link(
            LinkTarget::Uri("https://example.com/report".into()),
            Some(RUN_LINK.into()),
        )
        .tag(TagKind::P);
    s.tag(TagKind::P, |s| {
        line(s, "Our motto:", noto, 11.0, 56.0, 440.0)?;
        tagged(
            s,
            Tag::new(TagKind::Span).with_lang("fr"),
            "joie de vivre",
            noto,
            11.0,
            112.0,
            440.0,
        )
    })?;
    tagged(
        s,
        Tag::custom("Total", TagKind::P),
        "Total sales: € 4,300",
        noto,
        14.0,
        56.0,
        470.0,
    )
}

fn second_page(s: &mut Surface, noto: &Font) -> Res {
    s.artifact(ArtifactKind::Background, |s| {
        s.fill_color(Color::gray(0.97))
            .path(|p| {
                p.rect(rect(0.0, 0.0, 595.28, 841.89));
            })
            .fill(FillRule::NonZero)
            .fill_color(Color::gray(0.0));
        Ok(())
    })?;
    pagination(s, noto, 2)?;
    s.tag(TagKind::Sect, |s| {
        tagged(s, TagKind::H2, "Notes", noto, 18.0, 56.0, 96.0)?;
        tagged(s, TagKind::P, "Figures are unaudited.", noto, 11.0, 56.0, 120.0)?;
        tagged(s, TagKind::Quote, "“Best quarter yet.”", noto, 11.0, 56.0, 140.0)?;
        tagged(s, TagKind::Note, "1. Excluding returns.", noto, 9.0, 56.0, 160.0)?;
        tagged(s, TagKind::Code, "cargo xtask golden --update", noto, 10.0, 56.0, 180.0)
    })?;
    s.tag(TagKind::Link, |s| {
        line(s, "Back to page 1", noto, 11.0, 56.0, 210.0)?;
        s.link(
            rect(56.0, 198.0, 80.0, 16.0),
            LinkTarget::Page {
                index: 0,
                fit: lamina_build::Fit::Page,
            },
            Some(PAGE_LINK.into()),
        );
        Ok(())
    })
}

/// The report's builder, with both pages drawn (not finished: tests read its diagnostics).
pub fn tagged_report_builder() -> Builder {
    let mut b = Builder::new(
        BuildOptions::default()
            .with_id(*b"lamina-build-B8a")
            .with_producer("lamina-build golden")
            .with_creation_date("D:20261030120000Z".parse().unwrap())
            .with_compress(false)
            .with_tagged(true),
    );
    let noto = b.font(font("NotoSans-Regular.ttf")).unwrap();
    let mut page = b.page(PageSize::A4);
    first_page(page.surface(), &noto).unwrap();
    page.finish().unwrap();
    let mut page = b.page(PageSize::A4);
    second_page(page.surface(), &noto).unwrap();
    page.finish().unwrap();
    b.metadata(Metadata::default().with_title("Quarterly report"))
        .lang("en-GB");
    b
}

/// `fixtures/golden/tagged-report.pdf`.
pub fn tagged_report() -> Vec<u8> {
    save(tagged_report_builder().finish().unwrap())
}
