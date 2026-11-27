---
title: "pdfa-attachment-forbidden"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/pdfa-attachment-forbidden/"
---

# pdfa-attachment-forbidden

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file claims PDF/A-1 or PDF/A-2, and an edit attached a file to it. The attachment is in the document, but the file may no longer meet its PDF/A level.

## Why

PDF/A-1 allows no attached files at all. PDF/A-2 allows only files that are PDF/A themselves, and Lamina PDF can't check that an attached file is (ISO 19005-2 §6.8). PDF/A-3 is the level made for carrying other files, such as an e-invoice's XML.

## How to fix

Attach the file to a copy saved without the PDF/A claim, or undo the attachment. If you know the attached file is PDF/A-1 or PDF/A-2, the PDF/A-2 file stays valid; check it with a validator such as veraPDF. The warning never blocks the edit or the save.

## Example

Attaching `notes.txt` to a file whose XMP metadata has `pdfaid:part` 2, then `doc.plan()`, lists `pdfa-attachment-forbidden`.

## Data fields

- `attachments`: the IDs of the attached files.
