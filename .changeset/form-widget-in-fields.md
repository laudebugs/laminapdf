---
"@laminapdf/wasm": patch
"@laminapdf/sdk": patch
---

A form whose field list (`/Fields`) names a field's widgets instead of the field now lists that field, where it used to leave it out. `doc.formFields()` (`form` in Rust) follows each such widget to its field, as pdf.js and qpdf do, and lists that field once with all its listed widgets.
