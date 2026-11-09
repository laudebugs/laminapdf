---
title: "missing-title"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/missing-title/"
---

# missing-title

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A PDF/UA-1 document (`Profile::PdfUa1`) reached `Builder::finish` without a title: `Builder::metadata` was never called, or its `Metadata` has no title, or the title is blank.

## Why

A screen reader announces the title first, so a document without one is hard to tell apart from others. The PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.1) requires a title in the document's metadata. The title is `dc:title` in the XMP packet, the XML metadata stream that the builder writes from `Metadata`. The standard also asks viewers to show the title in the title bar instead of the file name (`/DisplayDocTitle`, which the builder writes for you). This is condition 06-003 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures.

## How to fix

Give the document a title that says what it is.

## Example

```rust
builder.metadata(Metadata::default().with_title("Quarterly report, Q3 2026"));
```

## Data fields

None.
