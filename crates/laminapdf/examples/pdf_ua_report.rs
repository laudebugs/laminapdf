//! An accessible report that screen readers can follow, checked against PDF/UA-1 when it is built.

use laminapdf::{
    ArtifactKind, BuildError, BuildOptions, Builder, Color, FillRule, FontHandle, LinkTarget, Metadata, PageSize,
    PointPt, Profile, Pt, RectPt, SaveOptions, Surface, TableScope, Tag, TagKind,
};

const MARGIN: f64 = 56.0;
const ROW: f64 = 18.0;
const COLUMN: f64 = 144.0;
const HEADER_AT: PointPt = PointPt::from_points(MARGIN, 40.0);
const TITLE_BASELINE: f64 = 90.0;
const TABLE_TOP: f64 = 150.0;
const LINK_AT: PointPt = PointPt::from_points(MARGIN, 400.0);
const CHART_LEFT: f64 = 320.0;
const CHART_BASE: f64 = 360.0;
const BAR_WIDTH: Pt = Pt::new(36.0);
const BAR_PITCH: f64 = 50.0;
const BODY: Pt = Pt::new(11.0);
/// The font sizes of the title and the headings; other text is `BODY`.
const SIZES: [(TagKind, Pt); 2] = [(TagKind::H1, Pt::new(22.0)), (TagKind::H2, Pt::new(15.0))];
const BLUE: Color = Color::Rgb(0.12, 0.47, 0.78);
const QUARTERS: [(&str, f64); 4] = [("Q1", 120.0), ("Q2", 135.0), ("Q3", 160.0), ("Q4", 150.0)];
const CHART_ALT: &str = "Bar chart of sales by quarter: 120, 135, 160 and 150 thousand euros.";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let font = std::fs::read(args.next().ok_or("arguments: <font.ttf> [out-dir]")?)?;
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    Ok(std::fs::write(out.join("pdf_ua_report.pdf"), build(font)?)?)
}

pub(crate) fn build(font: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut b = Builder::new(BuildOptions::new().with_profile(Profile::PdfUa1));
    b.metadata(Metadata::default().with_title("Annual sales report 2026"));
    b.lang("en-GB");
    let font = b.font_bytes(font)?;
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font)?;
    page.finish()?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// One line of text with its baseline at `y`, as the structure element `tag`, sized by it.
fn line(s: &mut Surface, font: &FontHandle, tag: impl Into<Tag>, text: &str, x: f64, y: f64) -> Result<(), BuildError> {
    let tag = tag.into();
    let size = SIZES.iter().find(|e| e.0 == tag.kind()).map_or(BODY, |e| e.1);
    s.tag(tag, |s| s.text(text, font, size, PointPt::from_points(x, y)).map(drop))
}

fn draw(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    // The running header is decoration: an artifact, which screen readers skip.
    let header = |s: &mut Surface| s.text("Acme Widgets", font, BODY, HEADER_AT).map(drop);
    s.artifact(ArtifactKind::Pagination, header)?;
    line(s, font, TagKind::H1, "Annual sales report 2026", MARGIN, TITLE_BASELINE)?;
    let summary = "Sales grew in three quarters of four.";
    line(s, font, TagKind::P, summary, MARGIN, TITLE_BASELINE + 2.0 * ROW)?;
    table(s, font)?;
    chart(s)?;
    let link = LinkTarget::Uri("https://example.com/accounts".into());
    s.tag(TagKind::P, |s| {
        let text = s.text("The full accounts", font, BODY, LINK_AT)?;
        text.fill(BLUE).link(link, Some("The full 2026 accounts".into()));
        Ok(())
    })
}

/// A heading and a table of the quarters, whose first row holds the columns' header cells.
fn table(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    line(s, font, TagKind::H2, "Sales by quarter", MARGIN, TABLE_TOP)?;
    let header = Tag::new(TagKind::TH).with_scope(TableScope::Column);
    s.tag(TagKind::Table, |s| {
        s.tag(TagKind::TR, |s| {
            line(s, font, header.clone(), "Quarter", MARGIN, TABLE_TOP + ROW)?;
            line(s, font, header.clone(), "Sales (k€)", MARGIN + COLUMN, TABLE_TOP + ROW)
        })?;
        for (row, (quarter, sales)) in (2_u8..).zip(QUARTERS) {
            let y = TABLE_TOP + ROW * f64::from(row);
            s.tag(TagKind::TR, |s| {
                line(s, font, TagKind::TD, quarter, MARGIN, y)?;
                line(s, font, TagKind::TD, &format!("{sales:.0}"), MARGIN + COLUMN, y)
            })?;
        }
        Ok(())
    })
}

/// The quarters as bars standing on the chart's base: one figure with alternate text.
fn chart(s: &mut Surface) -> Result<(), BuildError> {
    s.tag(Tag::new(TagKind::Figure).with_alt(CHART_ALT), |s| {
        s.fill_color(BLUE);
        for (i, (_, sales)) in (0_u8..).zip(QUARTERS) {
            let x = CHART_LEFT + BAR_PITCH * f64::from(i);
            let bar = RectPt::from_xywh(Pt::new(x), Pt::new(CHART_BASE - sales), BAR_WIDTH, Pt::new(sales));
            s.path(|p| _ = p.rect(bar)).fill(FillRule::NonZero);
        }
        Ok(())
    })
}
