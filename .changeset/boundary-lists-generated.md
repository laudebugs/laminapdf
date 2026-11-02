---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
---

`@laminapdf/sdk` exports the `LineCap`, `LineJoin` and `TableScope` types that `StrokeStyle` and `TagSpec` use. The WASM module's `beginTag` and `beginArtifact` now take a structure type, table scope and artifact kind by name instead of by index; `@laminapdf/sdk` calls them that way already. `laminapdf-build`'s `FillRule`, `LineCap`, `LineJoin`, `BlendMode`, `ArtifactKind` and `TableScope` gain an `ALL` list of their variants.
