---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

Accessible and archival PDFs. A `profiles` option on the builder (`PDF/UA-1`, `PDF/A-2b`, `PDF/A-2u`, `PDF/A-3b`; `BuildOptions::with_profile` in Rust) checks each machine-checkable rule the document breaks when it finishes, with a coded error per rule: `missing-title`, `missing-lang`, `untagged-content`, `missing-alt-text`, `heading-level-skipped`, `invalid-structure-nesting`, `link-without-structure`, `link-without-contents`, `font-not-embedded`, `glyph-without-unicode`, `device-color-without-output-intent`, `embedded-file-forbidden` and `profile-version-conflict`. The builder writes the XMP identification, `/DisplayDocTitle` for PDF/UA-1 and, for PDF/A, an sRGB output intent with a 480-byte embedded ICC profile. `attach(name, bytes, mime, { relationship })` adds associated files for PDF/A-3b, such as an e-invoice's XML. Every example that claims a profile is validated by veraPDF each night.
