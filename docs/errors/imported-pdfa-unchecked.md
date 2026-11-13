# `imported-pdfa-unchecked`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to a PDF/A level, and pages were copied in from a document that doesn't claim the same level. Saving the file as it is may write one that no longer meets the claim.

## Why

PDF/A limits what a file may hold: every font embedded, colours tied to an output intent, no transparency at PDF/A-1, and more (ISO 19005-2 §6). A page copied from another document brings its fonts, images and colour spaces with it. When that document claims the same PDF/A level, they were already checked against it. When it claims another level or none, nothing checks them, and Lamina PDF can't tell whether they keep the claim.

## How to fix

Check the saved file with a PDF/A validator such as veraPDF, or copy pages from a document that claims the same level. If the file needn't stay PDF/A, save anyway: the warning never blocks a save.

## Example

A PDF/A-2b file merges in a scanned report that claims no PDF/A level with `mergeDocuments`. Its `doc.plan()` then lists `imported-pdfa-unchecked`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E2d](../todos/done/E2d-merge-and-split.md): merging and splitting documents
- [E2 plan §3](../plans/editor/e2-page-organisation.md#3-conformance-the-edits-must-keep): what page edits do to a PDF/A claim
