---
title: "pdfua-title-missing"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/pdfua-title-missing/"
---

# pdfua-title-missing

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/UA-1, which requires a title in its XMP metadata (`dc:title`, ISO 14289-1 §7.1), and the metadata has none, usually because an edit removed it. Saving it as it is writes a file that no longer meets PDF/UA-1.

## Why

A screen reader announces a document by its title, so PDF/UA requires one.

## How to fix

Give the document a title (`setMetadata({ title: "…" })`) before saving. If the file needn't stay PDF/UA-1, save anyway: the warning never blocks a save.

## Example

`doc.removeMetadata(["title"])` on a PDF/UA-1 file, then `doc.plan()`, lists `pdfua-title-missing`.

## Data fields

None.
