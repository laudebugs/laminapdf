---
"@laminapdf/sdk": patch
---

Every export of `@laminapdf/sdk` now says which release added it: a `@since 0.2.0` tag, which the API reference shows as "Since" and editors show on hover. In Rust, every public item's docs end with "Added in 0.2.0." Items added later will name the release they first ship in. The types `ExtractFormat`, `ReadingOrder`, `SplitOn`, `TableMode`, `LayoutRegion`, `ModelInfo`, `ModelRequest`, `Models`, `OcrWord` and `RegionLabel` are now declared in `@laminapdf/sdk` with their own docs; they're the same types as before.
