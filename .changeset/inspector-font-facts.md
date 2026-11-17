---
"@laminapdf/wasm": patch
---

The font inspector reads more of what a font dictionary says. It now names a font whose `/BaseFont` is an indirect object, and a Type 3 font by its descriptor's `/FontName` or its `/Name`, with the subset tag read from that name. A font has a ToUnicode CMap only when `/ToUnicode` is a stream, as text extraction reads it. A simple font's encoding dictionary without `/Differences` shows as its base encoding instead of a custom one.
