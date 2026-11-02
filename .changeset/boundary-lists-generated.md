---
"@laminapdf/sdk": patch
"@laminapdf/wasm": minor
---

`@laminapdf/sdk` exports the `LineCap`, `LineJoin` and `TableScope` types that `StrokeStyle` and `TagSpec` use. Breaking for code that calls the WASM module directly: its `beginTag` and `beginArtifact` now take a structure type, table scope and artifact kind by name instead of by index. `@laminapdf/sdk` already calls them that way. `laminapdf-build`'s `FillRule`, `LineCap`, `LineJoin`, `BlendMode`, `ArtifactKind` and `TableScope` gain an `ALL` list of their variants.
