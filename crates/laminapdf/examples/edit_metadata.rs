//! Edit a document's title, authors and keywords, keeping its Info dictionary and XMP metadata in step.
//!
//! One edit sets the title and the authors, another removes the keywords, and the save stamps
//! the time it happened. Before saving, the plan says whether the save is allowed (a certified
//! file forbids metadata changes) and which PDF/A or PDF/UA claims the edit breaks.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_metadata -- in.pdf <save-time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Document, MetadataKey, MetadataPatch, Op, OpenOptions, PdfDate, RemoveMetadata, SaveMode, SaveOptions, SetMetadata,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: edit_metadata in.pdf <save-time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // The time of the save, which your app takes from its clock.
    let saved_at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?, saved_at)?;
    std::fs::write(out.join("edit_metadata.pdf"), edited)?;
    Ok(())
}

/// The document with its metadata edited, saved incrementally at `saved_at`.
pub(crate) fn edit(bytes: Vec<u8>, saved_at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;

    let patch = MetadataPatch::new()
        .title(Some("Quarterly figures"))
        .authors(&["Ada Lovelace", "Charles Babbage"]);
    doc.apply(vec![
        Op::from(SetMetadata::new(patch, Some(saved_at))),
        Op::from(RemoveMetadata::new(vec![MetadataKey::Keywords])),
    ])?;

    let options = SaveOptions::new()
        .with_mode(SaveMode::Incremental)
        .with_metadata_date(saved_at);
    let plan = doc.plan(&options)?;
    if let Some(refusal) = plan.refusal {
        return Err(format!("the save would be refused: {}", refusal.as_str()).into());
    }
    for warning in &plan.conformance_warnings {
        eprintln!("warning: {}", warning.code());
    }
    Ok(doc.save(&options)?.bytes)
}
