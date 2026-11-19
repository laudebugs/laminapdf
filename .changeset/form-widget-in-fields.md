---
"@laminapdf/wasm": patch
"@laminapdf/sdk": patch
---

A form whose field list names a field's widgets instead of the field now lists that field. `doc.formFields()` (`form` in Rust) follows each such widget to the field it belongs to and lists the field once, with its name, value and every listed widget, as pdf.js and qpdf do. It used to leave the field out.
