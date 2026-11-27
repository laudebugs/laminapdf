# `content-font-invalid`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op adds text to a page in a font whose bytes Lamina PDF can't read as a TrueType or OpenType font.

## Why

Lamina PDF embeds the part of each font a page uses, so it has to read the font's tables. Bytes that are truncated, in another format (a WOFF2 file, a web font kit), or not a font at all, can't be read, so the text can't be drawn in them.

## How to fix

Give the bytes of a TrueType (`.ttf`) or OpenType (`.otf`) font file, added to the session as a blob, or use `helvetica`, `times` or `courier`, which need no bytes.

## Example

```json
{ "op": "addText", "args": { "id": "note", "page": "p1.0", "rect": [72, 72, 288, 96], "runs": [{ "text": "Hello" }], "fonts": ["9f2c…"], "modified": "D:20261118120000Z" } }
```

The blob `9f2c…` holds a WOFF2 file. Convert it to TrueType, or pick another font.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E6c](../todos/ongoing/E6c-add-text.md): text and special characters added as page content
