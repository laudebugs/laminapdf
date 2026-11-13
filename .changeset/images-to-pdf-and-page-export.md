---
"@laminapdf/sdk": minor
"@laminapdf/viewer": minor
"@laminapdf/wasm": minor
---

Turn images into a PDF, and export pages as PNG, JPEG or TIFF:

- `imagesToPdf(images, options)` (`laminapdf::images_to_pdf` in Rust) makes a document with one page per image. JPEGs are embedded as they are, and each image is drawn upright, as its EXIF orientation says. Pages take the image's size at its resolution, or a paper size (`fit: "A4"`, or `[width, height]` in points) with the image fitted inside the margins. In Rust, `Image::orientation()` reads the orientation.
- In the web reader's worker, `ViewerClient.exportPage(index, { format, dpi, quality })` draws a page as PNG or JPEG, and `exportTiff(indexes, { dpi, multiPage })` as TIFF, every page in one file or one file each. The resolution is recorded in each file. The TIFF writer is a WebAssembly module of its own, `@laminapdf/wasm/tiff`, loaded the first time a TIFF export starts.
- In Rust, `laminapdf_render::export_pages` (feature `export`) draws pages one at a time and encodes them as PNG, JPEG or TIFF at a resolution you choose, and `TiffWriter` (feature `tiff`) writes a TIFF from pixels you drew yourself, a page at a time.
- `export-failed` is a new error code: a page export's encoder failed.
