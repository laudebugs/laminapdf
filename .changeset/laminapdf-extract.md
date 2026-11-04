---
"@laminapdf/sdk": minor
---

Two new warning codes, `extract-limit-blocks` and `extract-reading-order-ambiguous`, join the generated `WarningCode` union. In Rust, `Budget`, `Step` and `Phase` now come from `laminapdf-syntax` (the façade re-exports them at the same paths), `laminapdf_model::Document::is_tagged` and `RectPt::union` are new, and `laminapdf_text::is_right_to_left` is public. A new crate, `laminapdf-extract`, extracts blocks in reading order (ADR 0030); it is published with L3b.
