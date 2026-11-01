# `missing-title`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A PDF/UA-1 document (`Profile::PdfUa1`) reached `Builder::finish` without a title: `Builder::metadata` was never called, or its `Metadata` has no title, or the title is blank.

## Why

PDF/UA-1, the standard for accessible PDF (ISO 14289-1), in §7.1 needs a `dc:title` in the document's XMP packet, and asks viewers to show it in the title bar instead of the file name (`/DisplayDocTitle`, which the builder writes for you). A screen reader announces the title first, so a document without one is hard to tell apart from others (Matterhorn Protocol 06-003; the protocol is the PDF Association's checklist of PDF/UA failures).

## How to fix

Give the document a title that says what it is.

## Example

```rust
builder.metadata(Metadata::default().with_title("Quarterly report, Q3 2026"));
```

## Data fields

None.
