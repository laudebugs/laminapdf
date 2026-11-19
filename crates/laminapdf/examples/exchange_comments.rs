//! Send a PDF's comments to a reviewer who uses another app, and bring their comments back:
//! export them as XFDF, which Acrobat and most PDF apps import, and import the XFDF file they
//! send back. The import keeps the comments already in the file and adds the new ones, with
//! their replies and statuses, as one change that a single undo takes back.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example exchange_comments -- report.pdf reply.xfdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    CommentFormat, Document, ExportOptions, ImportMode, ImportOptions, ImportWarning, OpenOptions, SaveOptions,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: exchange_comments report.pdf reply.xfdf [out-dir]";
    let pdf = PathBuf::from(args.next().ok_or(usage)?);
    let reply = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let bytes = std::fs::read(&pdf)?;
    std::fs::write(out.join("comments.xfdf"), export_comments(bytes.clone(), "report.pdf")?)?;
    std::fs::write(
        out.join("exchange_comments.pdf"),
        import_comments(bytes, &std::fs::read(reply)?)?,
    )?;
    Ok(())
}

/// The document's comments as XFDF, recording the file's name so an import can tell when it is
/// given to another PDF.
pub(crate) fn export_comments(bytes: Vec<u8>, name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let mut options = ExportOptions::new(CommentFormat::Xfdf);
    options.file = Some(name.to_owned());
    Ok(doc.export_annotations(&options)?)
}

/// The document with a reviewer's XFDF or FDF comments added to its own.
pub(crate) fn import_comments(bytes: Vec<u8>, xfdf: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let (summary, _) = doc.import_annotations(xfdf, &ImportOptions::new(ImportMode::Merge))?;
    eprintln!("{} comments added, {} already here", summary.added, summary.matched);
    // A sound, a form field's value or a comment for a missing page is left out, and said.
    for warning in &summary.warnings {
        if let ImportWarning::Skipped(skipped) = warning {
            eprintln!("{}: {} ({})", warning.code(), skipped.kind, skipped.reason.as_str());
        } else {
            eprintln!("{}", warning.code());
        }
    }
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
