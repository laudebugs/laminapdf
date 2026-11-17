---
"@laminapdf/wasm": patch
---

In Rust, `SignatureKind::name` and `Restriction::name` return the names a save plan's signature kinds and restrictions take in JSON (`certification`, `docMdp` and so on), so every binding writes the same ones.
