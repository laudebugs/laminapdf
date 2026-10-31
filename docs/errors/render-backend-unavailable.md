# `render-backend-unavailable`

- Type: error
- Kind: `io` (reading or writing outside lamina failed)

## What happened

The PDFium backend (`lamina-render-pdfium`) could not load the PDFium library, so nothing can be rendered with it.

## Why

PDFium is bound at runtime (ADR 0004 §3), never linked at build time: the library has to be present on the machine. Development machines and CI fetch the pinned build with `cargo xtask fetch-pdfium`; shipped apps bundle it.

## How to fix

Run `cargo xtask fetch-pdfium`, or set `LAMINA_PDFIUM_DIR` to the folder that holds the PDFium library (`libpdfium.dylib`, `libpdfium.so` or `pdfium.dll`).

## Example

```rust
let backend = PdfiumBackend::from_env()?; // render-backend-unavailable when the library isn't there
```

## Data fields

- `backend`: `pdfium`.
