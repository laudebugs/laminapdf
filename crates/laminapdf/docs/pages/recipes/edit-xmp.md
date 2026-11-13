---
title: "Record your own metadata"
description: "Add properties of your own to a PDF's XMP metadata, such as a review's status and reviewers, and keep a PDF/A file valid while you do."
version: "0.3.0"
url: "https://docs.laminapdf.com/recipes/edit-xmp/"
---

# Record your own metadata

Add properties of your own to a PDF's XMP metadata, such as a review's status and reviewers, and keep a PDF/A file valid while you do.

You'll record a review in a PDF's XMP metadata: its status, its reviewers and the review round, as properties of your own. You need this when a workflow tracks facts a PDF has no field for, such as an approval, a case number or a retention date.

XMP metadata is the XML copy of a PDF's metadata. Besides the title and authors, it holds any property a namespace defines, and a namespace is a URI you own, so your names can't clash with anyone else's. This example edits an archive-ready PDF/A file:

**TypeScript**

`packages/sdk/examples/edit-xmp.ts`:

```ts
// Record a review in a document's XMP metadata: properties of your own, beyond the title and
// authors, which a PDF/A file keeps without losing its claim.
// Each property is named by a namespace you own and a local name. In a PDF/A file, Lamina PDF also
// declares each new property in the metadata's extension schemas, as PDF/A requires, and the edit
// lists what it declared.
import { Document } from "@laminapdf/sdk";

/** The namespace of the review properties: a URI you own, so they can't clash with anyone else's. */
const REVIEW = "https://example.com/ns/review/1.0/";

/**
 * The document with the review recorded in its XMP metadata, saved incrementally.
 *
 * @param savedAt - the time of the save, as a PDF date (`D:20261110093000Z`) your app takes
 *   from its clock.
 */
export function editXmp(bytes: Uint8Array, savedAt: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  const reviewers = [
    { type: "text", value: "Ada Lovelace" },
    { type: "text", value: "Charles Babbage" },
  ] as const;
  const edits = [
    doc.setXmpProperty(REVIEW, "review", "status", { type: "text", value: "Approved" }),
    doc.setXmpProperty(REVIEW, "review", "reviewers", { type: "seq", value: [...reviewers] }),
    doc.setXmpProperty(REVIEW, "review", "round", { type: "integer", value: 2 }),
  ];
  for (const property of edits.flatMap((edited) => edited.declared)) {
    console.warn(`declared ${property.prefix}:${property.name} as ${property.valueType}`);
  }

  return doc.save({ mode: "incremental", metadataDate: savedAt });
}
```

**Rust**

`crates/laminapdf/examples/edit_xmp.rs`:

````rust
//! Record a review in a document's XMP metadata: properties of your own, beyond the title and
//! authors, which a PDF/A file keeps without losing its claim.
//!
//! Each property is named by a namespace you own and a local name. In a PDF/A file, Lamina PDF
//! also declares each new property in the metadata's extension schemas, as PDF/A requires, and
//! the edit lists what it declared.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_xmp -- in.pdf <save-time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, Op, OpenOptions, PdfDate, SaveMode, SaveOptions, SetXmpProperty, XmpValue};

/// The namespace of the review properties: a URI you own, so they can't clash with anyone else's.
const REVIEW: &str = "https://example.com/ns/review/1.0/";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: edit_xmp in.pdf <save-time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // The time of the save, which your app takes from its clock.
    let saved_at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?, saved_at)?;
    std::fs::write(out.join("edit_xmp.pdf"), edited)?;
    Ok(())
}

/// The document with the review recorded in its XMP metadata, saved incrementally at `saved_at`.
pub(crate) fn edit(bytes: Vec<u8>, saved_at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;

    let reviewers = XmpValue::Seq(vec![
        XmpValue::Text("Ada Lovelace".into()),
        XmpValue::Text("Charles Babbage".into()),
    ]);
    let edited = doc.apply(vec![
        Op::from(SetXmpProperty::new(
            REVIEW,
            "review",
            "status",
            XmpValue::Text("Approved".into()),
        )),
        Op::from(SetXmpProperty::new(REVIEW, "review", "reviewers", reviewers)),
        Op::from(SetXmpProperty::new(REVIEW, "review", "round", XmpValue::Integer(2))),
    ])?;
    for property in &edited.declared {
        eprintln!(
            "declared {}:{} as {}",
            property.prefix, property.name, property.value_type
        );
    }

    let options = SaveOptions::new()
        .with_mode(SaveMode::Incremental)
        .with_metadata_date(saved_at);
    Ok(doc.save(&options)?.bytes)
}
````

**Result:** [edit_xmp.pdf](https://docs.laminapdf.com/examples/pdf/edit_xmp.pdf)

The result is the original bytes followed by one revision. Its XMP metadata holds `review:status`, `review:reviewers` and `review:round`, and still meets PDF/A-2b.

## How it works

- `setXmpProperty(namespace, prefix, name, value)` (`SetXmpProperty` in Rust) sets one property. `prefix` is the short name the XML writes for the namespace, used when the metadata has none for it yet.
- A value carries its type: `{ type: "text", value: "Approved" }`. The types are `text`, `uri`, `integer`, `real`, `boolean`, `date` (a PDF date), `langAlt` (the same text in several languages), and `seq` and `bag`, ordered and unordered lists of one type.
- PDF/A allows only the properties its standard names, unless the metadata declares the others in an extension schema. Lamina PDF writes that declaration for each new property, and the edit's `declared` lists them.
- A file without XMP metadata gets it with the first property you set.

## What to change

- **Remove a property:** `removeXmpProperty(namespace, name)`. As with any removal, an [incremental save](../../pages/reference/glossary.md#incremental-save) keeps the old value in the file's earlier revisions; save with `mode: "fullRewrite"` to drop it.
- **Title, authors and dates:** use [`setMetadata`](../../pages/recipes/edit-metadata.md). `setXmpProperty` on `dc:title` works too and keeps the Info dictionary in step, but only with the value type the field needs: text for the title, a `seq` of text for the authors.
- **Structures and comments:** read the metadata with `xmpPacket()`, change its XML, and write it back with `replaceXmpPacket(xml)`. XML that doesn't parse is refused with [`invalid-xmp-packet`](../../errors/invalid-xmp-packet.md), and nothing changes.
- **The PDF/A or PDF/UA claim itself:** an edit can't change it ([`xmp-claim-read-only`](../../errors/xmp-claim-read-only.md)). Only converting or checking the file makes that claim.

Next: [save a PDF](../../pages/guides/save-a-pdf.md), for what a signed or repaired file allows.
