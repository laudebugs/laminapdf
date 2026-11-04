---
"@laminapdf/sdk": patch
---

`didYouMean` now suggests the same names as errors from the WASM module. Names equally close to a mistyped one come in name order, and a typo with characters outside ASCII counts one edit per character.
