---
title: "Edit a tagged PDF"
description: "Edit the text and objects of an accessible, tagged PDF with its tags kept up to date, and check its PDF/A and PDF/UA claims before you save."
version: "0.6.0"
url: "https://docs.laminapdf.com/recipes/edit-tagged-pdf/"
---

# Edit a tagged PDF

Edit the text and objects of an accessible, tagged PDF with its tags kept up to date, and check its PDF/A and PDF/UA claims before you save.

You'll edit an accessible report that claims PDF/UA and save it with its claim still true. Lamina PDF keeps the file's tags up to date as you edit, so screen readers still read the page in its order, and before you save it tells you which claims your edits broke.

The program deletes the report's summary paragraph and renames its table's heading. It then asks the save plan which claims no longer hold, and removes only those claims before saving:

**TypeScript**

`packages/sdk/examples/edit-tagged-pdf.ts`:

```ts
// Edit an accessible (tagged, PDF/UA) document and keep its claim, or save without it.
//
// Deletes the summary paragraph of the report from `pdf-ua-report` and renames its table's
// heading. Lamina PDF keeps the tags in step: the deleted paragraph leaves the structure tree,
// and the heading's element still finds its text. Before saving, the plan lists the PDF/A and
// PDF/UA claims the edits broke; this example removes only those claims, so the file doesn't
// promise what it no longer meets.
import { Document } from "@laminapdf/sdk";

/** The report with its summary deleted and its table's heading renamed. Needs `init({ extract: true })`. */
export function editTaggedPdf(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const runs = doc.content.textRuns(0);
  const summary = runs.find((r) => r.text.startsWith("Sales grew"));
  const heading = runs.find((r) => r.text === "Sales by quarter");
  if (!summary || !heading) {
    throw new Error("not the report from pdf-ua-report");
  }
  // Each edit names its run as the page is before it: the heading comes later on the page, so
  // renaming it first leaves the summary's name as it was.
  doc.content.editText(0, heading.run, "Quarterly sales");
  // Empty text deletes the run, and the paragraph it was.
  doc.content.editText(0, summary.run, "");

  // A `pdfa-…` code (and `imported-pdfa-unchecked`) breaks the PDF/A claim; the others, the
  // PDF/UA claim.
  const broken = doc.plan().conformanceWarnings.map((w) => w.code);
  const pdfa = broken.some((code) => code.includes("pdfa"));
  const pdfua = broken.some((code) => !code.includes("pdfa"));
  if (pdfa || pdfua) {
    console.warn(`the edits broke a claim: ${broken.join(", ")}`);
    doc.removeConformanceClaims({ pdfa, pdfua });
  }
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/edit_tagged_pdf.rs`:

````rust
//! Edit an accessible (tagged, PDF/UA) document and keep its claim, or save without it.
//!
//! Deletes the summary paragraph of the report from `pdf_ua_report` and renames its table's
//! heading. Lamina PDF keeps the tags in step: the deleted paragraph leaves the structure tree,
//! and the heading's element still finds its text. Before saving, the plan lists the PDF/A and
//! PDF/UA claims the edits broke; this example removes only those claims, so the file doesn't
//! promise what it no longer meets.
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
    let heading = runs.iter().find(|r| r.text == "Sales by quarter").ok_or("no heading")?;
    // Each edit names its run as the page is before it: the heading comes later on the page, so
    // renaming it first leaves the summary's name as it was.
    doc.apply(vec![
        EditText::new(page.clone(), heading.run, "Quarterly sales".into()).into(),
        // Empty text deletes the run, and the paragraph it was.
        EditText::new(page, summary.run, String::new()).into(),
    ])?;

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
````

**Result:** [edit_tagged_pdf.pdf](https://docs.laminapdf.com/examples/pdf/edit_tagged_pdf.pdf)

In the result, the summary is gone and the heading reads "Quarterly sales". The file still meets PDF/UA-1, so it keeps its claim.

## How tags follow your edits

A tagged PDF marks each piece of a page's content with a number, and its structure tree says which element (a heading, a paragraph, a figure) each number belongs to. Lamina PDF updates both as you edit:

- **Deleted content leaves the tree.** Delete an object or a run of text, and its element no longer names it. An element left with no content leaves the tree too.
- **Moved and rewritten content keeps its place.** An object you move, resize or reorder, and text you rewrite, stay in their elements.
- **Text alternatives follow the text.** When you rewrite text whose element has a text alternative (`ActualText` or `Alt`) that holds the old words once, the new words replace them. An alternative that doesn't hold them is removed, so a screen reader reads the new text itself.
- **A replaced figure needs new alternative text.** The old text described the old image, so Lamina PDF removes it, and in a file that claims PDF/UA-1 the save plan warns with `figure-without-alt` until the figure has new text.

## Check the claims before you save

`doc.plan().conformanceWarnings` lists each PDF/A or PDF/UA claim the edits break. The warnings never block a save, so you choose what to do:

- **Fix it first:** undo the edit that broke the claim. For a replaced figure, you can instead add alternative text with an accessibility tool and open the file again. The warning goes when the problem does.
- **Save without the claim:** `doc.removeConformanceClaims({ pdfua: true })` takes the claim out of the file's XMP metadata, so the file no longer promises what it doesn't meet. Set only the claims the warnings name, as the example does. This edit is the only one that may remove a claim; the other metadata edits refuse with `xmp-claim-read-only`. An [incremental save](../../pages/reference/glossary.md#incremental-save) keeps the old claim in the file's earlier revisions; save with `mode: "fullRewrite"` to drop it.

Content edits can break a claim three ways: `figure-without-alt` for a replaced figure, `content-tags-broken` for content the page's tags don't cover, and `pdfa-font-not-embedded` for a font an edit added without embedding it (once edits can add fonts).

Next: [edit the text a page shows](../../pages/recipes/edit-text.md), or [a PDF/UA report](../../pages/recipes/pdf-ua-report.md) to make an accessible file from the start.
