//! Edit an accessible (tagged, PDF/UA) document and keep its claim, or save without it.
//!
//! Deletes the summary paragraph of the report from `pdf_ua_report` and renames its table's
//! heading. Lamina PDF keeps the tags in step: the deleted paragraph leaves the structure tree,
//! and the heading's element still finds its text. Before saving, the plan lists the PDF/A and
//! PDF/UA claims the edits broke; when there are any, this example removes those claims, so the
//! file doesn't promise what it no longer meets.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_tagged_pdf -- pdf_ua_report.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, EditText, OpenOptions, RemoveConformanceClaims, SaveOptions};

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

/// The report with its summary deleted and its table's heading renamed, saved with the claims
/// it still meets.
pub(crate) fn edit(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let page = doc.page_info(0)?.id;
    let runs = doc.text_runs(0)?;
    let summary = runs
        .iter()
        .find(|r| r.text.starts_with("Sales grew"))
        .ok_or("no summary")?;
    let heading = runs
        .iter()
        .find(|r| r.text == "Sales by quarter")
        .ok_or("no heading")?;
    // Each edit names its run as the page is before it: the heading comes later on the page, so
    // renaming it first leaves the summary's name as it was.
    doc.apply(vec![
        EditText::new(page.clone(), heading.run, "Quarterly sales".into()).into(),
        // Empty text deletes the run, and the paragraph it was.
        EditText::new(page, summary.run, String::new()).into(),
    ])?;

    let options = SaveOptions::new();
    let broken: Vec<&str> = doc
        .plan(&options)?
        .conformance_warnings
        .iter()
        .map(|w| w.code())
        .collect();
    if !broken.is_empty() {
        eprintln!("saving without the PDF/A and PDF/UA claims: {broken:?}");
        doc.apply(vec![RemoveConformanceClaims::new(true, true).into()])?;
    }
    Ok(doc.save(&options)?.bytes)
}
