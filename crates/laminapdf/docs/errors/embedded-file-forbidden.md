---
title: "embedded-file-forbidden"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/embedded-file-forbidden/"
---

# embedded-file-forbidden

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Builder::attach` added a file to a document built under PDF/A-2b or PDF/A-2u.

## Why

The PDF/A-2 standard for archival PDF (ISO 19005-2 §6.8) allows only attached files that are themselves PDF/A. Lamina PDF can't check what an attachment is, so it rejects every attachment under PDF/A-2 on purpose, PDF/A ones included. The PDF/A-3 standard for archival PDF (ISO 19005-3) allows files of any kind, provided they are associated with the document, which `Builder::attach` does.

## How to fix

Choose `Profile::PdfA3b` for documents that carry files, such as an e-invoice's XML, or leave the attachment out.

## Example

```rust
let options = BuildOptions::default().with_profile(Profile::PdfA2b);
let mut builder = Builder::new(options);
builder.attach(Attachment::new("data.csv", csv, "text/csv"));
builder.finish()?; // embedded-file-forbidden: use Profile::PdfA3b
```

## Data fields

- `profile`: the profile, such as `PDF/A-2b`.
