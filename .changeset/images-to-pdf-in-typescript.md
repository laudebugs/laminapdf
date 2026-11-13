---
"@laminapdf/sdk": minor
"@laminapdf/viewer": minor
"@laminapdf/wasm": minor
---

Turn images into a PDF and export pages as PNG, JPEG or TIFF from TypeScript, as Rust already can:

- `imagesToPdf(images, options)` makes a document with one page per image, as `laminapdf::images_to_pdf` does: JPEGs embedded as they are, each image drawn upright, and pages the image's size or a paper size (`fit: "A4"`, or `[width, height]` in points) with the image fitted inside the margins. It needs the extract or the text module (`init({ extract: true })`).
- In the web reader's worker, `ViewerClient.exportPage(index, { format, dpi, quality })` draws a page as PNG or JPEG, and `exportTiff(indexes, { dpi, multiPage })` as TIFF, every page in one file or one file each. The resolution is recorded in each file. The TIFF writer is a WebAssembly module of its own, `@laminapdf/wasm/tiff`, loaded the first time a TIFF export starts.
- In Rust, `laminapdf_render::TiffWriter` (feature `tiff`) writes a TIFF from pixels you drew yourself, a page at a time.
