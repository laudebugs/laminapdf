# `images-unavailable`

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

An edit replaces an image on a page, but the document has no image writer to turn the new JPEG or PNG into an image object.

## Why

A PDF stores an image as an image object: its pixels, their colour space and how they are compressed. Reading a JPEG or PNG and writing that object is the `laminapdf-build` crate's work. The `laminapdf` crate and the `@laminapdf/sdk` package always connect it, so only code that uses `laminapdf-ops` directly meets this error.

## How to fix

Edit through the `laminapdf` crate or `@laminapdf/sdk`. If you use `laminapdf-ops` directly, give the document an image writer first: `Document::with_images` with `laminapdf-build`'s `ImageXObjectWriter`.

## Example

```json
{ "op": "editContent", "args": { "page": "p3.0", "edits": [{ "replaceImage": { "object": { "ordinal": 2, "hash": "a3f1c2d4e5f60718" }, "blob": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" } }] } }
```

Applied to a `laminapdf_ops::Document` opened without `with_images`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E7d](../todos/done/E7d-select-transform-existing.md): replacing images
- [E7 plan §2.3](../plans/editor/e7-editing-existing-content.md#23-objects): what a replaced image writes
