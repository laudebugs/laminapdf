# `export-failed`

- Type: error
- Kind: `internal` (a bug in Lamina PDF)

## What happened

A page was drawn for an export, but the PNG, JPEG or TIFF encoder failed to write it.

## Why

The encoders write into memory, and the drawing gives them a valid image, so a failure is a bug in Lamina PDF or in the encoder, not a problem with the document. A page too big for the format is reported as `render-too-large` instead.

## How to fix

Report it with the format, the resolution and the page. Exporting in another format may work meanwhile.

## Example

```rust
let files = export_pages(doc.as_ref(), &[0], &ExportOptions::new(ExportFormat::Png), &CancelToken::new())?; // export-failed only on a bug
```

## Data fields

- `format`: `png`, `jpeg` or `tiff`.

## For contributors

The decisions and todos behind this code:

- [E3h](../todos/ongoing/E3h-images-to-pdf-and-page-export.md): page export
