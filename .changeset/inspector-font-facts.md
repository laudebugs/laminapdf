---
"@laminapdf/wasm": patch
---

The fonts that `properties().fonts` and the font inspector list now read three things as the file declares them:

- **Names.** A font whose `/BaseFont` is an indirect object now has its name, and a Type 3 font is named by its font descriptor's `/FontName`, else its `/Name`. The subset tag is read from that name.
- **ToUnicode.** `toUnicode` is true only for a `/ToUnicode` stream; a name such as `/Identity-H` maps nothing.
- **Encodings.** A simple font's encoding dictionary without `/Differences` now shows as its `/BaseEncoding` instead of a custom encoding.

Text extraction reads an indirect `/BaseEncoding` by the same rule, so it no longer falls back to StandardEncoding there.
