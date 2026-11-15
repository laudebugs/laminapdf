---
title: "Edit the metadata"
description: "Change a PDF's title, authors and keywords in both of the places a PDF keeps them, check the save first, and stamp when it happened."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/edit-metadata/"
---

# Edit the metadata

Change a PDF's title, authors and keywords in both of the places a PDF keeps them, check the save first, and stamp when it happened.

You'll change a PDF's title and authors, remove its keywords, and save the edit with the time it was made. You need this for a properties dialog, a batch that fixes titles, or a pipeline that marks reports final.

A PDF keeps these facts twice: in its Info dictionary, which older viewers read, and in its XMP metadata, an XML copy that archives and search engines read. Each edit writes both from one value:

**TypeScript**

`packages/sdk/examples/edit-metadata.ts`:

```ts
// Edit a document's title, authors and keywords, keeping its Info dictionary and XMP metadata in step.
// One edit sets the title and the authors, another removes the keywords, and the save stamps the
// time it happened. Before saving, the plan says whether the save is allowed (a certified file
// forbids metadata changes) and which PDF/A or PDF/UA claims the edit breaks.
import { Document } from "@laminapdf/sdk";

/**
 * The document with its metadata edited, saved incrementally.
 *
 * @param savedAt - the time of the save, as a PDF date (`D:20261110093000Z`) your app takes
 *   from its clock.
 */
export function editMetadata(bytes: Uint8Array, savedAt: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  doc.setMetadata({ title: "Quarterly figures", authors: ["Ada Lovelace", "Charles Babbage"] }, { at: savedAt });
  doc.removeMetadata(["keywords"]);

  const options = { mode: "incremental", metadataDate: savedAt } as const;
  const plan = doc.plan(options);
  if (plan.refusal !== undefined) {
    throw new Error(`the save would be refused: ${plan.refusal}`);
  }
  for (const warning of plan.conformanceWarnings) {
    console.warn(`warning: ${warning.code}`);
  }
  return doc.save(options);
}
```

**Rust**

`crates/laminapdf/examples/edit_metadata.rs`:

````rust
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
````

**Result:** [edit_metadata.pdf](https://docs.laminapdf.com/examples/pdf/edit_metadata.pdf)

The result is the original bytes, unchanged, followed by one revision. Both copies of the metadata name "Quarterly figures" and its two authors, neither holds keywords, and the modification date is the time of the save.

## How it works

- `metadata()` reads both copies at once. When they disagree, it trusts the XMP copy in a PDF 2.0 or PDF/A file and otherwise the copy changed last, and it lists the fields that differ in `disagreements`.
- `setMetadata` (`SetMetadata` in Rust) takes a patch: a value sets a field, `null` removes it, and a field you leave out stays as it was. Authors are a list. The Info dictionary holds them as one string, joined with `; `.
- `at` is the modification date the edit records, a PDF date such as `D:20261110093000Z` that your app takes from its clock. Leave it out and the edit writes no date.
- `removeMetadata` (`RemoveMetadata`) removes fields from both copies and leaves the rest of the XMP metadata as it was.
- `plan(options)` says how the save will go before it happens. On a certified file, `refusal` is `would-invalidate-signatures`, because a certification allows no metadata change. `conformanceWarnings` lists the PDF/A or PDF/UA rules the edit breaks, such as a title removed from an accessible file. The warnings never stop a save.
- `metadataDate` (`with_metadata_date`) is the time of the save, from your app's clock. The save writes it as the modification date in both copies and gives the XMP metadata a new instance ID. Lamina PDF never reads the clock, so a save given neither `metadataDate` nor `instanceId` stamps nothing.

## What to change

- **Only some fields:** leave the others out of the patch.
- **A PDF 2.0 file:** nothing. Lamina PDF writes only the two dates to its Info dictionary, which PDF 2.0 deprecates, and everything else to the XMP metadata.
- **XMP metadata that can't be read:** the edit refuses with [`malformed-xmp`](../../errors/malformed-xmp.md). Pass `replaceMalformed: true` to replace it with a new copy.
- **Removing values for good:** an [incremental save](../../pages/reference/glossary.md#incremental-save) keeps the old values in the file's earlier revisions. Save with `mode: "fullRewrite"` to drop them; the edit's `stillInEarlierRevisions` says when it matters.
- **Your own instance ID:** pass `instanceId` (`with_instance_id`), a UUID, to the save.

Next: [save a PDF](../../pages/guides/save-a-pdf.md), for what a signed or repaired file allows.
