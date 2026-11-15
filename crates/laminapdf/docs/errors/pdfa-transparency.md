---
title: "pdfa-transparency"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/pdfa-transparency/"
---

# pdfa-transparency

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A-1, and an annotation the edits added or changed is translucent, or blends with the page as a highlight does. Saving the file as it is writes one that no longer meets PDF/A-1.

## Why

PDF/A-1 predates transparency in PDF and forbids it (ISO 19005-1 §6.4). A highlight multiplies its colour with the text under it so the text stays legible, and that blending is transparency. Later parts of PDF/A, PDF/A-2 and PDF/A-3, allow it.

## How to fix

Mark the text another way, with an underline or a strike-out, or make the mark opaque. If the file needn't stay PDF/A-1, save anyway: the warning never blocks a save.

## Example

A highlight added to a PDF/A-1b file, then `doc.plan()`, lists `pdfa-transparency`.

## Data fields

- `annotations`: the IDs of the translucent or blending annotations, in page order.
