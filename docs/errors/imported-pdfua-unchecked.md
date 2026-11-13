# `imported-pdfua-unchecked`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/UA-1, and pages were copied in from a document that doesn't make that claim. Saving the file as it is may write one that no longer meets the claim.

## Why

PDF/UA-1 asks more of a file than tags: every image needs alternate text, headings must nest in order, tables need their headers, and the reading order must make sense (ISO 14289-1 §7). Pages copied from another document bring that document's tags with them, or no tags when it has none. When that document claims PDF/UA-1 too, its tags were made to meet these rules. When it doesn't, nothing checks them, and Lamina PDF can't tell whether the pages keep the claim.

## How to fix

Check the saved file with a PDF/UA checker such as veraPDF, or copy pages from a document that claims PDF/UA-1. If the file needn't stay PDF/UA, save anyway: the warning never blocks a save.

## Example

A PDF/UA-1 report copies in a tagged page from a draft that claims no standard with `insertPagesFrom`. Its `doc.plan()` then lists `imported-pdfua-unchecked`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E2o](../todos/done/E2o-structure-tree-merge.md): merging structure trees, for merged and imported pages
- [E2 plan §3](../plans/editor/e2-page-organisation.md#3-conformance-the-edits-must-keep): what page edits do to a PDF/UA claim
