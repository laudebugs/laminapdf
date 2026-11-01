//! An accessible report that screen readers can follow, checked against PDF/UA-1 when it is built.

use std::error::Error;

use lamina::{
    ArtifactKind, BuildError, BuildOptions, Builder, CancelToken, Color, FillRule, FontHandle, LinkTarget, Metadata,
    PageSize, PointPt, Profile, Pt, RectPt, SaveOptions, Surface, TableScope, Tag, TagKind,
};

const MARGIN: f64 = 56.0;
const ROW: f64 = 18.0;
const COLUMN: f64 = 144.0;
const BODY: Pt = Pt::new(11.0);
const TITLE: Pt = Pt::new(22.0);
const HEADING: Pt = Pt::new(15.0);
const BLUE: Color = Color::Rgb(0.12, 0.47, 0.78);
const QUARTERS: [(&str, f64); 4] = [("Q1", 120.0), ("Q2", 135.0), ("Q3", 160.0), ("Q4", 150.0)];
const CHART_ALT: &str = "Bar chart of sales by quarter: 120, 135, 160 and 150 thousand euros.";

fn main() -> Result<(), Box<dyn Error>> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("pdf_ua_report.pdf"), build()?)?;
    Ok(())
}

pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(BuildOptions::default().with_profile(Profile::PdfUa1));
    b.metadata(Metadata::default().with_title("Annual sales report 2026"));
    b.lang("en-GB");
    let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font)?;
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes)
}

/// One line of text with its baseline at `y`, as the structure element `tag`, sized by it.
fn line(s: &mut Surface, font: &FontHandle, tag: impl Into<Tag>, text: &str, x: f64, y: f64) -> Result<(), BuildError> {
    let tag = tag.into();
    let size = match tag.kind() {
        TagKind::H1 => TITLE,
        TagKind::H2 => HEADING,
        _ => BODY,
    };
    s.tag(tag, |s| s.text(text, font, size, PointPt::from_points(x, y)).map(drop))
}

fn draw(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    // The running header is decoration: an artifact, which screen readers skip.
    let (header, at) = (PointPt::from_points(MARGIN, 40.0), PointPt::from_points(MARGIN, 400.0));
    s.artifact(ArtifactKind::Pagination, |s| {
        s.text("Acme Widgets", font, BODY, header).map(drop)
    })?;
    line(s, font, TagKind::H1, "Annual sales report 2026", MARGIN, 90.0)?;
    let summary = "Sales grew in three quarters of four.";
    line(s, font, TagKind::P, summary, MARGIN, 90.0 + 2.0 * ROW)?;
    table(s, font, 150.0)?;
    chart(s, 360.0)?;
    let link = LinkTarget::Uri("https://example.com/accounts".into());
    s.tag(TagKind::P, |s| {
        let text = s.text("The full accounts", font, BODY, at)?;
        drop(text.fill(BLUE).link(link, Some("The full 2026 accounts".into())));
        Ok(())
    })
}

/// A heading and a table of the quarters, whose first row holds the columns' header cells.
fn table(s: &mut Surface, font: &FontHandle, top: f64) -> Result<(), BuildError> {
    line(s, font, TagKind::H2, "Sales by quarter", MARGIN, top)?;
    let header = Tag::new(TagKind::TH).with_scope(TableScope::Column);
    s.tag(TagKind::Table, |s| {
        s.tag(TagKind::TR, |s| {
            line(s, font, header.clone(), "Quarter", MARGIN, top + ROW)?;
            line(s, font, header.clone(), "Sales (k€)", MARGIN + COLUMN, top + ROW)
        })?;
        for (row, (quarter, sales)) in (2_u8..).zip(QUARTERS) {
            let y = top + ROW * f64::from(row);
            s.tag(TagKind::TR, |s| {
                line(s, font, TagKind::TD, quarter, MARGIN, y)?;
                line(s, font, TagKind::TD, &format!("{sales:.0}"), MARGIN + COLUMN, y)
            })?;
        }
        Ok(())
    })
}

/// The quarters as bars standing on `base`: one figure with alternate text.
fn chart(s: &mut Surface, base: f64) -> Result<(), BuildError> {
    s.tag(Tag::new(TagKind::Figure).with_alt(CHART_ALT), |s| {
        s.fill_color(BLUE);
        for (i, (_, sales)) in (0_u8..).zip(QUARTERS) {
            let x = 320.0 + 50.0 * f64::from(i);
            let bar = RectPt::from_xywh(Pt::new(x), Pt::new(base - sales), Pt::new(36.0), Pt::new(sales));
            s.path(|p| _ = p.rect(bar)).fill(FillRule::NonZero);
        }
        Ok(())
    })
}
