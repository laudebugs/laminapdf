---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Rust only for now: turn images into a PDF, and export pages as PNG, JPEG or TIFF. The TypeScript functions follow.

- `laminapdf::images_to_pdf(&images, &ImagesOptions)` makes a document with one page per image. JPEGs are embedded as they are, and each image is drawn upright, as its EXIF orientation says. Pages take the image's size at its resolution, or a paper size with the image fitted inside the margins (`ImagePages::Fit`). `Image::orientation()` reads the orientation.
- `laminapdf_render::export_pages` (feature `export`) draws pages one at a time and encodes them as PNG, JPEG or TIFF at a resolution you choose, with the resolution recorded in each file. A multi-page TIFF holds every page in one file.
