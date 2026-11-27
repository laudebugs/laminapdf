---
title: "pdfa-attachment-incomplete"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/pdfa-attachment-incomplete/"
---

# pdfa-attachment-incomplete

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file claims PDF/A-3, and an edit attached a file without something PDF/A-3 asks of every attached file: an association with the document, a page or an annotation, a modification date, or a MIME type.

## Why

ISO 19005-3 §6.8 lets a PDF/A-3 file carry any file, as long as each one says what it belongs to (an `/AF` array), when it last changed (`/ModDate`) and what type it is (`/Subtype`). Without them the file no longer meets PDF/A-3.

## How to fix

Attach the file to the document, a page or an annotation, with its modification date and MIME type: set `attachTo`, `modified` and `mime` in `attachFile`. The warning never blocks the edit or the save.

## Example

```json
{ "op": "attachFile", "args": { "name": "data.csv", "file": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08", "mime": "text/csv" } }
```

Applied to a PDF/A-3 file, then `doc.plan()`, lists `pdfa-attachment-incomplete`: the file is only in the document's list, and has no modification date.

## Data fields

- `attachments`: the IDs of the attached files that lack something.
