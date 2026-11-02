# @laminapdf/wasm

## 0.2.0-alpha.2

### Minor Changes

- 32a4963: The WASM module's `beginTag` and `beginArtifact` now take a structure type, table scope and artifact kind by name instead of by index, which breaks code that calls the module directly; `@laminapdf/sdk` already calls them that way. `@laminapdf/sdk` also exports the `LineCap`, `LineJoin` and `TableScope` types that `StrokeStyle` and `TagSpec` use, and `laminapdf-build`'s `FillRule`, `LineCap`, `LineJoin`, `BlendMode`, `ArtifactKind` and `TableScope` gain an `ALL` list of their variants.

## 0.2.0-alpha.1

### Minor Changes

- 5c2624e: The library is called Lamina PDF, and its public names follow. `@laminapdf/sdk`'s error class is `LaminaPdfError`, checked with `isLaminaPdfError`, and its warnings are `LaminaPdfWarning`. The error object's JSON Schema is `laminapdf.error/1`. `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro` (`.js`, `.d.ts` and `.wasm`).

## 0.2.0-alpha.0

### Minor Changes

- 2b4384a: Accessible and archival PDFs. A `profiles` option on the builder (`PDF/UA-1`, `PDF/A-2b`, `PDF/A-2u`, `PDF/A-3b`; `BuildOptions::with_profile` in Rust) checks each machine-checkable rule the document breaks when it finishes, with a coded error per rule: `missing-title`, `missing-lang`, `untagged-content`, `missing-alt-text`, `heading-level-skipped`, `unnumbered-heading-misused`, `invalid-structure-nesting`, `link-without-structure`, `link-without-contents`, `font-not-embedded`, `glyph-without-unicode`, `device-color-without-output-intent`, `embedded-file-forbidden`, `embedded-file-without-mime-type` and `profile-version-conflict`. The builder writes the XMP identification, `/DisplayDocTitle` for PDF/UA-1 and, for PDF/A, an sRGB output intent with a 480-byte embedded ICC profile. `attach(name, bytes, mime, { relationship })` adds associated files for PDF/A-3b, such as an e-invoice's XML. Every example that claims a profile is validated by veraPDF each night.

## 0.1.0-alpha.0

### Minor Changes

- First alpha, `0.1.0-alpha.0`, versioned and tagged but not published (ADR 0017). The `lamina` façade and its core crates at the same version: open, read, edit and save PDFs (incremental and full rewrites, the save planner, undo/redo), generate them (pages, vectors, colour, images, standard 14 and embedded fonts with shaping, bidi and line breaking, links, outlines, metadata, tagged PDF), and extract page text; `@laminapdf/sdk` over the base, text and extract WebAssembly modules in browsers, Node, Deno, Bun and Cloudflare Workers.
