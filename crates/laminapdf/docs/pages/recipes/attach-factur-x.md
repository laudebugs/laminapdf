---
title: "Make an invoice Factur-X"
description: "Attach an invoice's structured data to a PDF/A-3 invoice as factur-x.xml, and record the Factur-X properties, so the file stays valid PDF/A-3."
version: "0.7.0"
url: "https://docs.laminapdf.com/recipes/attach-factur-x/"
---

# Make an invoice Factur-X

Attach an invoice's structured data to a PDF/A-3 invoice as factur-x.xml, and record the Factur-X properties, so the file stays valid PDF/A-3.

You'll turn a PDF/A-3 invoice into a Factur-X one: the same file, carrying the invoice's data as XML for the buyer's software to read. You need this for e-invoicing in France and Germany, where Factur-X and ZUGFeRD are one format.

The function opens the invoice, attaches `factur-x.xml` as the document's alternative form, records the four Factur-X properties in the XMP metadata, and saves:

**TypeScript**

`packages/sdk/examples/attach-factur-x.ts`:

```ts
// Make a PDF/A-3 invoice a Factur-X (ZUGFeRD) one: attach its structured data as factur-x.xml,
// the document's alternative form, and record the Factur-X properties in the XMP metadata, which
// keeps the file valid PDF/A-3.
// The XML here is a placeholder: a real one is the invoice in the EN 16931 Cross Industry Invoice
// syntax, made by your invoicing system. The file must already be PDF/A-3.
import { Document } from "@laminapdf/sdk";

/** The Factur-X XMP namespace and prefix (Factur-X 1.0, §6.2.2). */
const FX = "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#";

/** A placeholder for the invoice's Cross Industry Invoice XML. */
const XML = `<?xml version="1.0" encoding="UTF-8"?>
<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100">
  <!-- The invoice's data, from your invoicing system. -->
</rsm:CrossIndustryInvoice>
`;

/**
 * The invoice with factur-x.xml attached and the Factur-X properties recorded, saved incrementally.
 *
 * @param savedAt - the time of the save, as a PDF date (`D:20261110093000Z`) your app takes
 *   from its clock.
 */
export function attachFacturX(bytes: Uint8Array, savedAt: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  // PDF/A-3 asks every attached file for an association, a MIME type and a modification date.
  const xml = doc.attachFileOp("factur-x.xml", new TextEncoder().encode(XML), "text/xml", {
    attachTo: "document",
    relationship: "alternative",
    description: "Factur-X invoice",
    modified: savedAt,
  });
  doc.transact((tx) => {
    tx.apply({ op: "attachFile", args: xml });
    const fx = (name: string, value: string) => tx.setXmpProperty(FX, "fx", name, { type: "text", value });
    fx("DocumentType", "INVOICE");
    fx("DocumentFileName", "factur-x.xml");
    fx("Version", "1.0");
    fx("ConformanceLevel", "MINIMUM");
  });

  // An attachment that breaks the file's PDF/A level says so before the save; this one keeps it.
  for (const warning of doc.plan({ mode: "incremental", metadataDate: savedAt }).conformanceWarnings) {
    console.warn(warning.code);
  }
  return doc.save({ mode: "incremental", metadataDate: savedAt });
}
```

**Rust**

`crates/laminapdf/examples/attach_factur_x.rs`:

````rust
//! Make a PDF/A-3 invoice a Factur-X (ZUGFeRD) one: attach its structured data as
//! factur-x.xml, the document's alternative form, and record the Factur-X properties in the XMP
//! metadata, which keeps the file valid PDF/A-3.
//!
//! The XML here is a placeholder: a real one is the invoice in the EN 16931 Cross Industry
//! Invoice syntax, made by your invoicing system. The file must already be PDF/A-3 (as the
//! `pdf_a3_invoice_with_xml` example writes).
//!
//! ```sh
//! cargo run -p laminapdf --example attach_factur_x -- invoice.pdf <save-time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AfRelationship, AttachTarget, Document, Op, OpenOptions, PdfDate, SaveMode, SaveOptions, SetXmpProperty, XmpValue,
};

/// The Factur-X XMP namespace and prefix (Factur-X 1.0, §6.2.2).
const FX: &str = "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#";

/// A placeholder for the invoice's Cross Industry Invoice XML.
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<rsm:CrossIndustryInvoice xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\">
  <!-- The invoice's data, from your invoicing system. -->
</rsm:CrossIndustryInvoice>
";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: attach_factur_x invoice.pdf <save-time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // The time of the save, which your app takes from its clock.
    let saved_at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let invoice = attach(std::fs::read(input)?, saved_at)?;
    std::fs::write(out.join("attach_factur_x.pdf"), invoice)?;
    Ok(())
}

/// The invoice with `factur-x.xml` attached and the Factur-X properties recorded, saved
/// incrementally at `saved_at`.
pub(crate) fn attach(bytes: Vec<u8>, saved_at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;

    // PDF/A-3 asks every attached file for an association, a MIME type and a modification date.
    let mut xml = doc
        .attach_file("factur-x.xml", XML.as_bytes().to_vec(), "text/xml")
        .with_target(AttachTarget::Document, AfRelationship::Alternative);
    xml.description = Some("Factur-X invoice".into());
    xml.modified = Some(saved_at);
    let fx = |name: &str, value: &str| Op::from(SetXmpProperty::new(FX, "fx", name, XmpValue::Text(value.into())));
    doc.apply(vec![
        Op::from(xml),
        fx("DocumentType", "INVOICE"),
        fx("DocumentFileName", "factur-x.xml"),
        fx("Version", "1.0"),
        fx("ConformanceLevel", "MINIMUM"),
    ])?;

    let options = SaveOptions::new()
        .with_mode(SaveMode::Incremental)
        .with_metadata_date(saved_at);
    // An attachment that breaks the file's PDF/A level says so before the save; this one
    // keeps it.
    for warning in doc.plan(&options)?.conformance_warnings {
        eprintln!("{}", warning.code());
    }
    Ok(doc.save(&options)?.bytes)
}
````

**Result:** [attach_factur_x.pdf](https://docs.laminapdf.com/examples/pdf/attach_factur_x.pdf)

The result is the original bytes followed by one revision. It lists `factur-x.xml` beside the invoice's own XML and still meets PDF/A-3b.

## How it works

- `attachFileOp(name, bytes, mime, options)` keeps the bytes for the session and makes the `attachFile` edit; `attach_file` does the same in Rust. `attachFile` applies it at once.
- `attachTo: "document"` associates the file with the whole document, and `relationship: "alternative"` says it is another form of the same content. PDF/A-3 asks every attached file for an association, a MIME type and a modification date (`modified`), which your app takes from its clock.
- Each file records its size, its dates and an MD5 checksum. Reading it back with `attachmentBytes` compares the checksum, so a file changed after it was attached shows as changed.
- The `fx:` properties are not part of PDF/A, so Lamina PDF declares them in the metadata's extension schema, as for any [property of your own](../../pages/recipes/edit-xmp.md).
- On a file that claims PDF/A-1 or PDF/A-2, `plan()` warns [`pdfa-attachment-forbidden`](../../errors/pdfa-attachment-forbidden.md): those levels allow no attached files but PDF/A ones.

## What to change

- **The XML:** replace the placeholder with the invoice in the EN 16931 Cross Industry Invoice syntax, from your invoicing system. Keep the name `factur-x.xml`; ZUGFeRD 1 files use `ZUGFeRD-invoice.xml`.
- **The profile:** set `ConformanceLevel` to the profile your XML follows: `MINIMUM`, `BASIC WL`, `BASIC`, `EN 16931` or `EXTENDED`.
- **A file that isn't PDF/A-3 yet:** make it with the builder, as [A PDF/A-3 invoice with XML](../../pages/recipes/pdf-a3-invoice-with-xml.md) does. Converting another file to PDF/A is a separate job.

Next: [Attachments](../../pages/guides/attachments.md), for reading, saving out and deleting attached files.
