//! An accessible report (PDF/UA-1, todo B8b): an embedded font only, a title and a language,
//! headings in order, a paragraph, a list, a table with header cells, a chart as a `Figure`
//! with alternate text, a described link, and the running header and the rules as artifacts.
//! `Profile::PdfUa1` checks all of it when the build finishes and writes the PDF/UA
//! identification and `/DisplayDocTitle`.
//!
//! ```sh
//! cargo run -p lamina --example pdf_ua_report -- [out-dir]   # writes <out-dir>/pdf_ua_report.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use lamina::{
    ArtifactKind, BuildOptions, Builder, CancelToken, Color, FillRule, LinkTarget, Metadata, PageSize, PointPt,
    Profile, Pt, RectPt, SaveOptions, StrokeStyle, TableScope, Tag, TagKind,
};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("pdf_ua_report.pdf"), build()?)?;
    Ok(())
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

/// Sales per quarter, drawn as bars and listed in the table.
const QUARTERS: [(&str, f64); 4] = [("Q1", 120.0), ("Q2", 135.0), ("Q3", 160.0), ("Q4", 150.0)];

/// The document's bytes.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = builder()?.finish()?;
    Ok(doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes)
}

/// The report, drawn and not yet finished (its tests read the tagging diagnostics).
pub(crate) fn builder() -> Result<Builder, Box<dyn Error>> {
    let at = PointPt::from_points;
    let options = BuildOptions::default()
        .with_id(*b"lamina-ex-pdfua1")
        .with_producer("lamina examples")
        .with_creation_date("D:20261031120000Z".parse()?)
        .with_profile(Profile::PdfUa1);
    let mut b = Builder::new(options);
    b.metadata(Metadata::default().with_title("Annual sales report 2026"))
        .lang("en-GB");
    let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    // The running header and the rules are decoration: artifacts, skipped by screen readers.
    s.artifact(ArtifactKind::Pagination, |s| {
        let _ = s.text("Acme Widgets Ltd · Annual report", &font, Pt::new(8.0), at(56.0, 40.0))?;
        s.stroke_color(Color::gray(0.6))
            .path(|p| {
                p.move_to(at(56.0, 48.0)).line_to(at(539.0, 48.0));
            })
            .stroke(&StrokeStyle::default());
        Ok(())
    })?;
    let _ = s
        .text("Annual sales report 2026", &font, Pt::new(22.0), at(56.0, 90.0))?
        .tag(TagKind::H1);
    let _ = s
        .text(
            "Sales grew in every quarter but the last.",
            &font,
            Pt::new(11.0),
            at(56.0, 115.0),
        )?
        .tag(TagKind::P);
    let _ = s
        .text("Highlights", &font, Pt::new(15.0), at(56.0, 150.0))?
        .tag(TagKind::H2);
    s.tag(TagKind::L, |s| {
        for (i, item) in (0_u8..).zip(["Three new distributors", "A second warehouse"]) {
            let y = 172.0 + 18.0 * f64::from(i);
            s.tag(TagKind::LI, |s| {
                let _ = s.text("•", &font, Pt::new(11.0), at(60.0, y))?.tag(TagKind::Lbl);
                let _ = s.text(item, &font, Pt::new(11.0), at(74.0, y))?.tag(TagKind::LBody);
                Ok(())
            })?;
        }
        Ok(())
    })?;
    let _ = s
        .text("Sales by quarter", &font, Pt::new(15.0), at(56.0, 230.0))?
        .tag(TagKind::H2);
    s.tag(TagKind::Table, |s| {
        s.tag(TagKind::TR, |s| {
            for (x, label) in [(56.0, "Quarter"), (200.0, "Sales (k€)")] {
                let _ = s
                    .text(label, &font, Pt::new(11.0), at(x, 255.0))?
                    .tag(Tag::new(TagKind::TH).with_scope(TableScope::Column));
            }
            Ok(())
        })?;
        for (row, (quarter, sales)) in (0_u8..).zip(QUARTERS) {
            let y = 275.0 + 18.0 * f64::from(row);
            s.tag(TagKind::TR, |s| {
                let _ = s.text(quarter, &font, Pt::new(11.0), at(56.0, y))?.tag(TagKind::TD);
                let _ = s
                    .text(&format!("{sales:.0}"), &font, Pt::new(11.0), at(200.0, y))?
                    .tag(TagKind::TD);
                Ok(())
            })?;
        }
        Ok(())
    })?;
    let chart = Tag::new(TagKind::Figure)
        .with_alt("Bar chart of sales by quarter: 120, 135, 160 and 150 thousand euros from Q1 to Q4.");
    s.tag(chart, |s| {
        s.fill_color(Color::rgb(0.12, 0.47, 0.78));
        for (i, (_, sales)) in (0_u8..).zip(QUARTERS) {
            let x = 320.0 + 50.0 * f64::from(i);
            s.path(|p| {
                p.rect(rect(x, 360.0 - sales, 36.0, sales));
            })
            .fill(FillRule::NonZero);
        }
        Ok(())
    })?;
    let _ = s
        .text(
            "Read the full accounts at example.com/accounts",
            &font,
            Pt::new(11.0),
            at(56.0, 400.0),
        )?
        .fill(Color::rgb(0.12, 0.47, 0.78))
        .link(
            LinkTarget::Uri("https://example.com/accounts".into()),
            Some("The full 2026 accounts on example.com".into()),
        )
        .tag(TagKind::P);
    page.finish()?;
    Ok(b)
}
