---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Tell XFA forms apart. `doc.xfa()` returns a form's XFA part, or `undefined` when it has none: its packets (`template`, `datasets`, `config` and the rest) as bytes, a `kind` of `"static"` or `"dynamic"`, and the `signals` that made it dynamic. A form is dynamic when the catalog's `/NeedsRendering` is true, when its `config` packet requires dynamic rendering, or when the template has fields and the AcroForm has none. The `removeXfa` edit takes the XFA part and `/NeedsRendering` out, so every viewer shows the AcroForm fields; undo brings the packets back byte for byte. A certified file and a file with usage rights don't allow it, and the save planner says so before the edit. In the Rust library, `Document::xfa` and the `RemoveXfa` op do the same.
