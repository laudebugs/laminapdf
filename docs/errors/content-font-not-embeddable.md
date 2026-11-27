# `content-font-not-embeddable`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op adds text to a page in a font whose licence says it must not be embedded in documents.

## Why

A font file declares in its `OS/2` table (`fsType`, ISO 32000-2 §9.9) whether it may be embedded. Added text is kept in the page as a subset of its font, so a font that forbids embedding can't hold it without breaking its licence. Lamina PDF refuses the font rather than embed it.

## How to fix

Use a font whose licence allows embedding: one of the fonts the reader offers, or `helvetica`, `times` or `courier`, which every reader has and need no embedding. If you own the font and its licence does allow embedding despite what the file declares, ask its maker for a corrected file.

## Example

```json
{ "op": "addText", "args": { "id": "note", "page": "p1.0", "rect": [72, 72, 288, 96], "runs": [{ "text": "Hello" }], "fonts": ["4b1e…"], "modified": "D:20261118120000Z" } }
```

The font in blob `4b1e…` has `fsType` 2 (restricted licence embedding). Choose another font.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E6c](../todos/done/E6c-add-text.md): text and special characters added as page content
