---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Running headers, footers and watermarks can now say what they are. `surface.artifact("footer", s => …)` marks its content as a footer attached to the bottom of the page (`/Subtype /Footer /Attached [/Bottom]`), `"header"` as a header attached to the top, and `"watermark"` as a watermark. `"pagination"` still writes the plain form. In Rust it is `Artifact::footer()`, `Artifact::header()` and `Artifact::watermark()`, with `with_attached` for other edges.

A paragraph split by a page break can now stay one paragraph for screen readers. In Rust, `surface.open_element(TagKind::P)` places it, `surface.resume(id, |s| …)` draws its content on that page or a later one, and `builder.close_element(id)` writes it.

`surface.measure_text_box(…)` (Rust) returns the layout `text_box` would draw, without drawing it.

Arabic text with diacritics now builds under PDF/UA-1 and PDF/A-2u: a letter the font draws as a body and a dot no longer fails with `glyph-without-unicode`.
