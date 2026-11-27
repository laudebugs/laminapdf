---
title: "Exchange comments with other apps"
description: "Send a PDF's comments to a reviewer who uses Acrobat or another app as XFDF, and bring their comments back as one change that a single undo takes back."
version: "0.8.0"
url: "https://docs.laminapdf.com/recipes/exchange-comments/"
---

# Exchange comments with other apps

Send a PDF's comments to a reviewer who uses Acrobat or another app as XFDF, and bring their comments back as one change that a single undo takes back.

You'll send a report's comments to a reviewer who uses another PDF app, and bring their answers back into your copy. You need this when people review the same document in different apps, or when the comments should travel without the PDF.

`exportAnnotations` writes the comments as XFDF, the format Acrobat and most PDF apps import. `importAnnotations` reads the XFDF or FDF file the reviewer sends back and adds its comments to the document as one change:

**TypeScript**

`packages/sdk/examples/exchange-comments.ts`:

```ts
// Send a PDF's comments to a reviewer who uses another app, and bring their comments back:
// export them as XFDF, which Acrobat and most PDF apps import, and import the XFDF file they
// send back. The import keeps the comments already in the file and adds the new ones, with
// their replies and statuses, as one change that a single undo takes back.
import { Document } from "@laminapdf/sdk";

/**
 * The document's comments as XFDF, recording the file's name so an import can tell when it is
 * given to another PDF. Needs `init({ extract: true })`.
 */
export function exportComments(bytes: Uint8Array, name: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  return doc.exportAnnotations({ format: "xfdf", file: name });
}

/** The document with a reviewer's XFDF or FDF comments added to its own. */
export function importComments(bytes: Uint8Array, xfdf: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const { summary } = doc.importAnnotations(xfdf, { mode: "merge" });
  console.log(`${summary.added} comments added, ${summary.matched} already here`);
  // A sound, a form field's value or a comment for a missing page is left out, and said.
  for (const warning of summary.warnings) {
    console.warn(warning.code, warning.kind ?? "", warning.reason ?? "");
  }
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/exchange_comments.rs`:

````rust
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
    CommentFormat, Document, ExportCommentsOptions, ImportCommentsOptions, ImportMode, ImportWarning, OpenOptions,
    SaveOptions,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: exchange_comments report.pdf reply.xfdf [out-dir]";
    let pdf = PathBuf::from(args.next().ok_or(usage)?);
    let reply = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let bytes = std::fs::read(&pdf)?;
    let name = pdf
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    std::fs::write(out.join("comments.xfdf"), export_comments(bytes.clone(), &name)?)?;
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
    let mut options = ExportCommentsOptions::new(CommentFormat::Xfdf);
    options.file = Some(name.to_owned());
    Ok(doc.export_annotations(&options)?)
}

/// The document with a reviewer's XFDF or FDF comments added to its own.
pub(crate) fn import_comments(bytes: Vec<u8>, xfdf: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let (summary, _) = doc.import_annotations(xfdf, &ImportCommentsOptions::new(ImportMode::Merge))?;
    eprintln!("{} comments added, {} already here", summary.added, summary.matched);
    // A sound, a form field's value or a comment for a missing page is left out, and said.
    for warning in &summary.warnings {
        if let ImportWarning::Skipped(skipped) = warning {
            eprintln!("{}: {} ({})", warning.code(), skipped.kind, skipped.cause.as_str());
        } else {
            eprintln!("{}", warning.code());
        }
    }
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

The comments already in your copy stay as they are. The reviewer's new comments, replies and statuses join them, each drawn the way Lamina PDF draws its own, and one undo takes them all back.

## How it works

- XFDF and FDF keep every reply with the comment it answers, carry attached files, and record which PDF they were made for. Importing a file made for another PDF still works, but the summary warns with `interchange-document-mismatch`: its comments may land in the wrong places. The check compares the two PDFs' identifiers, or, when either has none, the file name you pass as `file`.
- The import matches comments by their name (`/NM`) on the same page. By default, a comment the file names that is already in your copy is kept as it is, and the file's other comments are added.
- What Lamina PDF can't make is left out and listed in `summary.warnings` as `interchange-skipped`: a sound comment, a link, a form field's value, or a comment for a page your copy doesn't have.
- A file that doesn't read fails with `interchange-malformed`, which names the line where reading stopped. An XFDF file that declares a DTD is refused with `interchange-dtd`, since a DTD can define entities that expand without end or read other files.

## What to change

- **See what an import will do first:** `previewImport` gives the same summary without changing anything: how many comments it adds and on which pages, and what it leaves out.
- **Replace the comments the file names:** pass `{ mode: "replace" }`, and each one already in your copy is deleted, with its replies, and the file's version added.
- **Keep both:** pass `{ mode: "add" }`, and the file's comments are added as copies.
- **Notes for a notes app:** export `{ format: "markdown", comments: "highlightsAndNotes" }`. Each page gets a heading, and each highlight's text is quoted under it.
- **FDF instead of XFDF:** export `{ format: "fdf" }`, for apps that only read FDF.

Next: [answer a review](../../pages/recipes/review-comments.md) to reply to the comments you brought back.
