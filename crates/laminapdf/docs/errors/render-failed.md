---
title: "render-failed"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/render-failed/"
---

# render-failed

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The render engine (PDFium or hayro, `laminapdf-render`) could not open the document, or could not draw the page it was asked for.

## Why

The engines have their own parsers. A file that Lamina PDF repairs may still be too damaged for an engine, or use a feature it can't draw. An engine failure (for example a panic inside hayro) is reported this way too, never as a crash.

## How to fix

Check the document's warnings: a damaged file is usually reported there too. Try the other backend from the debug menu. If neither can draw the page, report the file with the backend name and version.

## Example

```rust
let doc = HayroBackend::new().open(Arc::new(bytes), &RenderOpenOptions::new())?; // render-failed for a file hayro can't parse
```

## Data fields

- `backend`: `pdfium` or `hayro`.
- `stage`: `open` or `render`.
