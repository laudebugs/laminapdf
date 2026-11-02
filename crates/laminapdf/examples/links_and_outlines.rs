//! Links and outlines: four A5 pages joined by links, bookmarks and page labels.
//!
//! The title page has a URI link, a link to a page and one through a named destination; every
//! other page has a linked text run back to it. The outline (bookmarks) is nested and open in
//! the viewer, and the pages are labelled i, ii, 1, 2.
//!
//! ```sh
//! cargo run -p laminapdf --example links_and_outlines -- [out-dir]   # writes <out-dir>/links_and_outlines.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    BuildOptions, Builder, Color, Fit, FontHandle, LabelStyle, LinkTarget, OutlineEntry, PageLabel, PageMode, PageSize,
    PointPt, Pt, RectPt, SaveOptions, Standard14, Surface, ViewerPreferences,
};

const TITLES: [&str; 4] = ["Title page", "Contents", "Chapter 1", "Chapter 2"];
const MARGIN: f64 = 36.0;
const TITLE_AT: PointPt = PointPt::from_points(MARGIN, 60.0);
const BACK_AT: PointPt = PointPt::from_points(MARGIN, 100.0);
const TITLE_SIZE: Pt = Pt::new(18.0);
const LINK_SIZE: Pt = Pt::new(12.0);
/// The baselines of the title page's three links.
const LINK_BASELINES: [f64; 3] = [100.0, 125.0, 150.0];
/// A link's clickable box: this wide, this tall, starting this far above the baseline.
const LINK_WIDTH: f64 = 220.0;
const LINK_HEIGHT: f64 = 20.0;
const LINK_RISE: f64 = 15.0;

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("links_and_outlines.pdf"), build(BuildOptions::new())?)?;
    Ok(())
}

/// The document's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options);
    let helvetica = b.standard_font(Standard14::Helvetica)?;
    b.destination("chapter-2", 3, Fit::Page);
    for (index, title) in (0_u32..).zip(TITLES) {
        let mut page = b.page(PageSize::A5);
        let s = page.surface();
        s.text(title, &helvetica, TITLE_SIZE, TITLE_AT)?;
        if index == 0 {
            title_page_links(s, &helvetica)?;
        } else {
            s.text("Back to the title page", &helvetica, LINK_SIZE, BACK_AT)?
                .fill(Color::rgb(0.0, 0.2, 0.8))
                .link(LinkTarget::page(0), Some("Title page".into()));
        }
        page.finish()?;
    }
    navigation(&mut b)?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

/// Where a link's text starts: the left margin, on `baseline`.
fn link_at(baseline: f64) -> PointPt {
    PointPt::from_points(MARGIN, baseline)
}

/// The title page's links: to a website, to a page, and to a named destination.
fn title_page_links(s: &mut Surface, font: &FontHandle) -> Result<(), Box<dyn Error>> {
    let site = LinkTarget::Uri("https://example.com/".into());
    let links = [
        ("Visit example.com", site, Some("Example")),
        ("Go to chapter 1", LinkTarget::page(2), None),
        ("Go to chapter 2 (named)", LinkTarget::Named("chapter-2".into()), None),
    ];
    for ((label, target, contents), baseline) in links.into_iter().zip(LINK_BASELINES) {
        s.text(label, font, LINK_SIZE, link_at(baseline))?;
        let area = RectPt::from_points(MARGIN, baseline - LINK_RISE, LINK_WIDTH, LINK_HEIGHT);
        s.link(area, target, contents.map(String::from));
    }
    Ok(())
}

/// The outline (bookmarks), the page labels, and the viewer opening with the outline shown.
fn navigation(b: &mut Builder) -> Result<(), Box<dyn Error>> {
    b.outline(vec![
        OutlineEntry::new("Title page", LinkTarget::page(0)),
        OutlineEntry::new("Contents", LinkTarget::page(1)).with_color(0.0, 0.0, 0.6),
        OutlineEntry::heading("Chapters")
            .open(true)
            .bold()
            .with_child(OutlineEntry::new("Chapter 1", LinkTarget::page(2)))
            .with_child(OutlineEntry::new("Chapter 2", LinkTarget::Named("chapter-2".into())).italic()),
    ])?;
    b.page_labels(vec![
        PageLabel::new(0, Some(LabelStyle::LowerRoman)),
        PageLabel::new(2, Some(LabelStyle::Decimal)),
    ])
    .viewer_preferences(ViewerPreferences::default().with_page_mode(PageMode::Outlines));
    Ok(())
}
