---
"@laminapdf/sdk": minor
---

Breaking (Rust, while alpha): every public type now has a name of its own, and each item one public path.

- An extracted run is `PageTextRun` (`laminapdf::extract::PageTextRun`, `laminapdf_text::PageTextRun`), the name `@laminapdf/sdk` already gives it. `TextRun` now means only drawn text.
- `laminapdf-build`'s font handle is `FontHandle`, the name the façade already gave it, so the builder's signatures read `&FontHandle` beside `laminapdf::Font`.
- `laminapdf-text`'s OpenType tag is `OpenTypeTag`, and the façade exports it for `Font::instance(&[(OpenTypeTag::WEIGHT, 700.0)])`.
- `laminapdf-text`'s `extract` and `agl` modules are private. Their items are at the crate root: `laminapdf_text::TextExtractor`, `PageText`, `Code` and the rest, `glyph_name_to_unicode` (was `agl::to_unicode`) and `zapf_dingbats_to_unicode`. `CMap` and `MAX_USECMAP_DEPTH` are now private to the crate.
- `laminapdf::Document::ops` and `into_ops` are hidden from the docs. They stay for Lamina PDF's own crates, and may change without notice.

In JavaScript, the extracted run's class is itself named `PageTextRun` now, so `run.constructor.name` reads `PageTextRun`. The rest of the JavaScript API and every output are unchanged.
