---
title: "Stamp a page and attach a file"
description: "Put an Approved stamp on a PDF page and attach a file to a spot on the page, drawn the same in every PDF reader, and play a PDF's sound comments."
version: "0.8.0"
url: "https://docs.laminapdf.com/recipes/stamp-and-attach/"
---

# Stamp a page and attach a file

Put an Approved stamp on a PDF page and attach a file to a spot on the page, drawn the same in every PDF reader, and play a PDF's sound comments.

You'll approve a report: a green stamp reading "APPROVED by Ada Lovelace" beside its title, and the table's figures attached as a CSV file next to the table. You need this for a review tool that signs off on documents, or a script that files a report's data inside it.

`stampAt` takes a box in page space and `attachFileAt` takes a point there, the way a viewer reports where a pointer went. Each gives the edit that adds the annotation:

**TypeScript**

`packages/sdk/examples/stamp-and-attach.ts`:

```ts
// Approve a report: a stamp with the approver's name at the top of its first page, and the
// table's figures attached as a CSV file beside the table.
// The box and the point are in page space (points from the page's top-left), as a viewer
// reports where a pointer went. The stamp's lines are set in Helvetica, which every reader has,
// so the WebAssembly extract module can draw it too.
import { type AnnotCommon, Document } from "@laminapdf/sdk";

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** The stamp, right of the title: `[x0, y0, x1, y1]`. */
const STAMP = [370, 56, 540, 106] as const;
/** The attachment's icon, right of the table's last column. */
const ICON = [370, 282] as const;
/** The table's figures. */
const FIGURES = "Quarter,Units,Sales\nQ1,120,1200\nQ2,310,3100\n";

/**
 * The document approved: a stamp and an attached file on its first page, saved incrementally.
 * `at` is when it was approved, as a PDF date.
 */
export function stampAndAttach(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = (): AnnotCommon => ({ author: "Ada Lovelace", created: at, modified: at });

  // The stamp's lines are written into it now and never change, as on a rubber stamp.
  const stamp = doc.stampAt(0, STAMP, { dynamic: { lines: ["APPROVED", "by Ada Lovelace"], style: "green" } });
  stamp.common = byMe();

  const attach = doc.attachFileAt(0, ICON, "figures.csv", new TextEncoder().encode(FIGURES), "text/csv");
  if (attach.spec.type === "fileAttachment") {
    attach.spec.icon = "graph";
    attach.spec.file.description = "The table's figures";
    attach.spec.file.modified = at;
  }
  attach.common = byMe();

  doc.apply([
    { op: "addAnnot", args: stamp },
    { op: "addAnnot", args: attach },
  ]);
  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/stamp_and_attach.rs`:

````rust
//! Approve a report: a stamp with the approver's name at the top of its first page, and the
//! table's figures attached as a CSV file beside the table.
//!
//! The box and the point are in page space (points from the page's top-left), as a viewer
//! reports where a pointer went. The stamp's lines are set in Helvetica, which every reader has,
//! so the WebAssembly extract module can draw it too.
//!
//! ```sh
//! cargo run -p laminapdf --example stamp_and_attach -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotCommon, AnnotSpec, AttachmentIcon, Document, DynamicStamp, Op, OpenOptions, PdfDate, PointPt, Pt, RectPt,
    SaveMode, SaveOptions, StampKind, StampStyle,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: stamp_and_attach in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the report was approved, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let approved = stamp_and_attach(std::fs::read(input)?, at)?;
    std::fs::write(out.join("stamp_and_attach.pdf"), approved)?;
    Ok(())
}

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// The stamp, right of the title: `[x0, y0, x1, y1]`.
const STAMP: [f64; 4] = [370.0, 56.0, 540.0, 106.0];
/// The attachment's icon, right of the table's last column.
const ICON: [f64; 2] = [370.0, 282.0];
/// The table's figures.
const FIGURES: &str = "Quarter,Units,Sales\nQ1,120,1200\nQ2,310,3100\n";

/// The document approved: a stamp and an attached file on its first page, saved incrementally.
pub(crate) fn stamp_and_attach(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let by_me = || {
        let mut common = AnnotCommon::default();
        common.author = Some("Ada Lovelace".into());
        common.created = Some(at);
        common.modified = Some(at);
        common
    };

    // The stamp's lines are written into it now and never change, as on a rubber stamp.
    let lines = vec!["APPROVED".to_owned(), "by Ada Lovelace".to_owned()];
    let approved = StampKind::Dynamic(DynamicStamp::new(lines, StampStyle::Green));
    let mut stamp = doc.stamp_at(0, area(STAMP), approved)?;
    stamp.common = by_me();

    let mut attach = doc.attach_file_at(0, point(ICON), "figures.csv", FIGURES.as_bytes().to_vec(), "text/csv")?;
    if let AnnotSpec::FileAttachment(spec) = &mut attach.spec {
        spec.icon = AttachmentIcon::Graph;
        spec.file.description = Some("The table's figures".into());
        spec.file.modified = Some(at);
    }
    attach.common = by_me();

    doc.apply(vec![Op::from(stamp), Op::from(attach)])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// A point in page space.
fn point([x, y]: [f64; 2]) -> PointPt {
    PointPt::new(Pt::new(x), Pt::new(y))
}

/// A box in page space from its corners.
fn area([x0, y0, x1, y1]: [f64; 4]) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}
````

**Result:** [stamp_and_attach.pdf](https://docs.laminapdf.com/examples/pdf/stamp_and_attach.pdf)

The result is the original bytes, unchanged, followed by one revision. The stamp sits right of the title, and a small bar-chart icon beside the table holds `figures.csv`.

## How it works

- A stamp shows one of three things. A **standard** stamp shows a name such as `approved`, `draft` or `confidential` in capitals inside a frame of its colour. An **image** stamp shows a JPEG or PNG fitted to its box. A **dynamic** stamp shows your own lines, the first in bold.
- Lamina PDF draws every stamp and icon itself and stores the drawing in the file, so every reader that draws stored appearances, as pdf.js and PDFium do, shows the same picture. A standard stamp's letters are drawn as shapes, so the stamp needs no font.
- A dynamic stamp's lines are fixed when you make the edit, as on a rubber stamp. Lamina PDF never runs a script to fill in a name or a time, so build the lines from your user's name and your clock.
- Lamina PDF stores the attached file's bytes exactly as given, with their size and a checksum. The document's attachments list it, held by the icon, and `attachmentBytes` gives the bytes back. Lamina PDF never opens an attached file.
- A screen reader reads each annotation's contents. Unless you give them, a stamp's contents are its name or its lines, and an attachment's are the file's name.

## What to change

- **A standard stamp:** pass `{ standard: "approved" }`, or another of the sixteen names, such as `notApproved`, `confidential` or `forComment`; the `StampName` type lists them all. Set the edit's `common.color` to draw it in another colour.
- **Your own image:** add a PNG or JPEG with `addBlob`, then pass `{ image: id }`. A PNG keeps its transparency. Other image formats fail the edit with [`stamp-image-invalid`](../../errors/stamp-image-invalid.md).
- **The icon:** set `spec.icon` to `"paperclip"`, `"pushPin"`, `"graph"` or `"tag"`. Change it later with `updateAnnot` and a `fileAttachment` patch.
- **In the browser:** standard and image stamps and attachments work in every module. A dynamic stamp needs the extract or text module; in the base module the edit fails with [`appearance-needs-text`](../../errors/appearance-needs-text.md).
- **A PDF/A file:** PDF/A-1 and PDF/A-2 forbid attached files, so `doc.plan()` warns with [`pdfa-attachment-forbidden`](../../errors/pdfa-attachment-forbidden.md). PDF/A-3 allows them with a modified date and a MIME type. A dynamic stamp's Helvetica isn't embedded, which PDF/A forbids, so `doc.plan()` warns with [`pdfa-font-not-embedded`](../../errors/pdfa-font-not-embedded.md); use a standard stamp there.
- **Sound comments:** older PDFs can hold recorded comments. `pageAnnotations` lists them with their format and length, and `soundAsWav(id)` returns the recording as a WAV file, which every browser's `<audio>` element plays.

Next: [review comments](../../pages/recipes/review-comments.md), to answer and resolve the comments on a page.
