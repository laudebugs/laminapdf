---
"@laminapdf/wasm": patch
---

In Rust, `SignatureKind::name` and `Restriction::name` give the names a save plan's signatures and restrictions have in JSON (`certification`, `docMdp` and so on), so every binding writes the same ones.
