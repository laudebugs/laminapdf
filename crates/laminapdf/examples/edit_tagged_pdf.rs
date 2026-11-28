//! Edit an accessible (tagged, PDF/UA) document and keep its claim, or save without it.
//!
//! Deletes the summary paragraph of the report from `pdf_ua_report`, renames its table's heading
//! and rewrites the alternative text of its chart. Lamina PDF keeps the tags in step: the deleted
//! paragraph leaves the structure tree, and the heading's element still finds its text. Before
//! saving, the plan lists the PDF/A and PDF/UA claims the edits broke; this example removes only
//! those claims, so the file doesn't promise what it no longer meets.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_tagged_pdf -- pdf_ua_report.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, EditText, OpenOptions, RemoveConformanceClaims, SaveOptions, SetAlt};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: edit_tagged_pdf pdf_ua_report.pdf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?)?;
    std::fs::write(out.join("edit_tagged_pdf.pdf"), edited)?;
    Ok(())
}

/// The report with its summary deleted, its table's heading renamed and its chart described anew,
/// saved with the claims it still meets.
pub(crate) fn edit(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let page = doc.page_info(0)?.id;
    let runs = doc.text_runs(0)?;
    let summary = runs
        .iter()
        .find(|r| r.text.starts_with("Sales grew"))
        .ok_or("no summary")?;
    let heading = runs.iter().find(|r| r.text == "Sales by quarter").ok_or("no heading")?;
    // Each edit names its run as the page is before it: the heading comes later on the page, so
    // renaming it first leaves the summary's name as it was.
    doc.apply(vec![
        EditText::new(page.clone(), heading.run, "Quarterly sales".into()).into(),
        // Empty text deletes the run, and the paragraph it was.
        EditText::new(page, summary.run, String::new()).into(),
    ])?;

    // The chart is the object that already has alternative text. Name it after the edits above,
    // since deleting the summary renumbers the page's objects. Setting its text changes it for
    // every object of the chart.
    let chart = doc
        .content_objects(0)?
        .into_iter()
        .find(|o| o.alt.is_some())
        .ok_or("no chart")?;
    let alt = "Bar chart of quarterly sales: 120, 135, 160 and 150 thousand euros.";
    doc.apply(vec![SetAlt::new(doc.page_info(0)?.id, chart.object, alt).into()])?;

    let options = SaveOptions::new();
    let mut remove = RemoveConformanceClaims::new();
    for warning in &doc.plan(&options)?.conformance_warnings {
        // A `pdfa-…` code (and `imported-pdfa-unchecked`) breaks the PDF/A claim; the others,
        // the PDF/UA claim.
        let code = warning.code();
        eprintln!("the edits broke a claim: {code}");
        remove = if code.contains("pdfa") {
            remove.pdfa()
        } else {
            remove.pdfua()
        };
    }
    if remove != RemoveConformanceClaims::new() {
        doc.apply(vec![remove.into()])?;
    }
    Ok(doc.save(&options)?.bytes)
}
