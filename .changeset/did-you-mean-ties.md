---
"@laminapdf/sdk": patch
---

`didYouMean` now suggests exactly what errors from the WASM module do: names equally close to a mistyped one come in name order, and a typo with characters outside ASCII is measured the same way, one edit per character.
