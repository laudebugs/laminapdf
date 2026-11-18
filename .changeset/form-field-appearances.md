---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

A filled form now shows its new values. The `setFieldValue` edit draws a text field's, combo box's or list box's widgets again in the field's own font, size, colour and alignment. Each widget keeps its own layout: text that fits its box, several lines, comb cells, a password's bullets, rows chosen in a list, or a turned widget. Check boxes and radio buttons switch between the states they already have, and one with no appearance gains a check mark or dot.

A character the field's font can't draw needs a font of your own. Add its bytes with `doc.addBlob` and list the ID in the edit's new `fonts`; the font is embedded with the form. Without one, the edit fails with `annotation-glyph-missing`. On a PDF/A file, that font draws the whole value, so the file keeps its claim. The base WebAssembly module has no fonts, so it refuses every text, combo and list fill with `appearance-needs-text`; the extract and text modules draw them.
