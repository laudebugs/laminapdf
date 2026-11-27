---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Add text to a page as page content, then change or remove it, even after the file is saved and reopened. Unlike a text box, which is a comment, the text is part of the page: it prints, every reader draws it, and it copies and searches as typed. `doc.content.addText(index, text)` takes a box in page space, the text as runs that can be bold or italic, and `fonts`: the font, then fallbacks for the characters it lacks (a symbol font, the font of another script). `"helvetica"`, `"times"` and `"courier"` need no bytes; any other font is added with `doc.addBlob` first, and Lamina PDF embeds the part of it the text uses. `doc.content.update(index, id, { text })` replaces the text and `doc.content.remove(index, id)` takes it off. In a tagged file the text is a paragraph, or decoration for `tag: "artifact"`.

Three new codes say what went wrong: `content-glyph-missing` (a character in none of the fonts), `content-font-not-embeddable` (a font whose licence forbids embedding) and `content-font-invalid` (bytes that aren't a font). The base module has no text layout, so the edit fails there with `appearance-needs-text`.
