---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

A filled form now shows its new values. The `setFieldValue` edit draws a text field's, combo box's or list box's widgets again in the field's own font, size, colour and alignment. Each widget keeps its own layout: text that fits its box, several lines, comb cells, a password's bullets, rows chosen in a list, or a turned widget. Check boxes and radio buttons switch between the states they already have.

A character the field's font can't draw needs a font of your own: add its bytes with `doc.addBlob` and list the ID in the edit's new `fonts`, and it is embedded with the form. Without one, the edit fails with `annotation-glyph-missing`. On a PDF/A file, that font draws the whole value, so the file keeps its claim. The base WebAssembly module, which has no fonts, fails such fills with `appearance-needs-text`; the extract and text modules draw them.
