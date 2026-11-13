---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Extract a document's images, links and bookmarks as files:

- `doc.extractImages(options)` (`extract_images` in Rust) gives the images the pages draw. JPEG and JPEG 2000 images come out as stored, byte for byte; every other image is decoded to a PNG with its `/Decode`, its colour space (grey, RGB, CMYK, Lab, Indexed, ICC-based, Separation) and its mask applied. Each image comes out once, on the first page that draws it, and one that can't be decoded is listed in `skipped` with why. In TypeScript it needs the extract or the text module; JBIG2 images, which the WebAssembly modules don't decode, are skipped there.
- `doc.linksJson()` and `doc.outlineJson()` give the links and the outline in the formats `laminapdf.links/1` and `laminapdf.outline/1`, with their JSON Schemas in the `laminapdf` crate; `linksCsv()` and `outlineCsv()` give them as CSV. In Rust, `Document::links()` lists every page's links.
- Streams filtered with `LZWDecode`, `ASCIIHexDecode`, `ASCII85Decode` or `RunLengthDecode` now decode: a page whose content uses one now has text, and its images come out.
