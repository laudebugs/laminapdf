# `content-needs-alt-text`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op adds an image, or a page of another PDF, to a file that claims PDF/UA-1, and gives it neither alternate text nor the mark that it is decoration.

## Why

PDF/UA-1 asks that every picture a screen reader meets has a description (ISO 14289-1 §7.3). Lamina PDF tags an added picture as a figure with its alternate text, or as an artifact, which screen readers skip, when it is decoration. Without either, the file would no longer meet its claim, so Lamina PDF refuses the op.

## How to fix

Give the object alternate text that says what it shows, or mark it as decoration, such as a border or a background pattern.

## Example

```json
{ "op": "addContentObject", "args": { "id": "chart", "page": "p3.0", "object": { "type": "image", "blob": "…" }, "matrix": [300, 0, 0, 200, 72, 300], "modified": "D:20261118120000Z" } }
```

The file claims PDF/UA-1, and the op has no `alt`. Add `"alt": { "text": "Sales by region, 2026" }`, or `"alt": "decorative"`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E6b](../todos/done/E6b-place-images-and-shapes.md): images, shapes, logos and stamps placed as page content
