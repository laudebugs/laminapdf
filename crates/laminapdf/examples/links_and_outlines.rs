//! Navigation: four A5 pages with a URI link, links to pages (forward and back) and through a
//! named destination, a linked text run, a nested outline (bookmarks) opened in the viewer,
//! and page labels i, ii, 1, 2.
//!
//! ```sh
//! cargo run -p laminapdf --example links_and_outlines -- [out-dir]   # writes <out-dir>/links_and_outlines.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    BuildOptions, Builder, Color, Fit, LabelStyle, LinkTarget, OutlineEntry, PageLabel, PageMode, PageSize, PointPt,
    Pt, RectPt, SaveOptions, Standard14, ViewerPreferences,
};

const TITLES: [&str; 4] = ["Title page", "Contents", "Chapter 1", "Chapter 2"];

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("links_and_outlines.pdf"), build()?)?;
    Ok(())
}

/// The document's bytes.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let at = PointPt::from_points;
    let row = |y: f64| RectPt::from_points(36.0, y - 15.0, 220.0, 20.0);
    let options = BuildOptions::default()
        .with_id(*b"lamina-ex-links!")
        .with_producer("lamina examples")
        .with_creation_date("D:20261030120000Z".parse()?);
    let mut b = Builder::new(options);
    let helvetica = b.standard_font(Standard14::Helvetica)?;
    b.destination("chapter-2", 3, Fit::Page);
    for (index, title) in (0_u32..).zip(TITLES) {
        let mut page = b.page(PageSize::A5);
        let s = page.surface();
        s.text(title, &helvetica, Pt::new(18.0), at(36.0, 60.0))?;
        if index == 0 {
            s.text("Visit example.com", &helvetica, Pt::new(12.0), at(36.0, 100.0))?;
            s.link(
                row(100.0),
                LinkTarget::Uri("https://example.com/".into()),
                Some("Example".into()),
            );
            s.text("Go to chapter 1", &helvetica, Pt::new(12.0), at(36.0, 125.0))?;
            s.link(row(125.0), LinkTarget::page(2), None);
            s.text("Go to chapter 2 (named)", &helvetica, Pt::new(12.0), at(36.0, 150.0))?;
            s.link(row(150.0), LinkTarget::Named("chapter-2".into()), None);
        } else {
            s.text("Back to the title page", &helvetica, Pt::new(12.0), at(36.0, 100.0))?
                .fill(Color::rgb(0.0, 0.2, 0.8))
                .link(LinkTarget::page(0), Some("Title page".into()));
        }
        page.finish()?;
    }
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
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
