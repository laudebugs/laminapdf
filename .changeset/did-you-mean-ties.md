---
"@laminapdf/sdk": patch
---

When several names are equally close to a mistyped one, `didYouMean` now lists them in name order, as errors from the WASM module already did, so a typo gets the same suggestions whichever layer reports it.
