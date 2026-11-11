---
title: "render-backend-unavailable"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/render-backend-unavailable/"
---

# render-backend-unavailable

- Type: error
- Kind: `io` (reading or writing outside Lamina PDF failed)

## What happened

The PDFium backend (`laminapdf-render-pdfium`) could not load the PDFium library, so nothing can be rendered with it.

## Why

PDFium is bound at runtime, never linked at build time: the library has to be present on the machine. Lamina PDF looks for it only in the folder you name.

## How to fix

Ship the PDFium library (`libpdfium.dylib`, `libpdfium.so` or `pdfium.dll`) with your app and pass its folder to `PdfiumBackend::bind(dir)`, or set `LAMINA_PDF_PDFIUM_DIR` to that folder for `PdfiumBackend::from_env()`.

## Example

```rust
let backend = PdfiumBackend::from_env()?; // render-backend-unavailable when the library isn't there
```

## Data fields

- `backend`: `pdfium`.
