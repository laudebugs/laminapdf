# `render-backend-unavailable`

- Type: error
- Kind: `io` (reading or writing outside lamina failed)

## What happened

The PDFium backend (`lamina-render-pdfium`) could not load the PDFium library, so nothing can be rendered with it.

## Why

PDFium is bound at runtime, never linked at build time: the library has to be present on the machine. lamina looks for it only in the folder you name.

## How to fix

Ship the PDFium library (`libpdfium.dylib`, `libpdfium.so` or `pdfium.dll`) with your app and pass its folder to `PdfiumBackend::bind(dir)`, or set `LAMINA_PDFIUM_DIR` to that folder for `PdfiumBackend::from_env()`.

## Example

```rust
let backend = PdfiumBackend::from_env()?; // render-backend-unavailable when the library isn't there
```

## Data fields

- `backend`: `pdfium`.

## For contributors

The decisions and todos behind this code:

- [ADR 0004](../decisions/0004-render-backend.md) §3

In a checkout of lamina, `cargo xtask fetch-pdfium` fetches the pinned build and prints its `LAMINA_PDFIUM_DIR`.
