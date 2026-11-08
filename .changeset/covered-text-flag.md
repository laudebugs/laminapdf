---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Extraction now leaves out text painted over by a later shape, image or annotation, as a reader can't see it either. A black box drawn over two words of a line takes those words out and keeps the rest of the line, so `Pay [box] now.` reads `Pay now.`. Covered text is listed under `dropped` with the new reason `covered`, its box and how many characters it held, never its text; `drop: { keepSpans: true }` shows it to an auditor. Only opaque paint covers: a translucent box, a highlighter, shading drawn before the text, a box in an optional layer and a scanned page's image over its own text layer leave the text in. In Rust, `Flags` gains `covered`, `DropReason` gains `Covered`, and `BlendMode` now lives in `laminapdf-model` (`laminapdf::BlendMode` is unchanged).
