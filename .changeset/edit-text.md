---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

You can now change the text a page already shows. `doc.content.textRuns(index)` (`text_runs` in Rust) lists a page's runs of text in the order the page draws them, each with its text, box, font, size and line, and a reason when it can't be edited. The new `editText` op (`doc.content.editText(index, run, text)`) gives a run new text in the font it is set in, and moves the text after it in the same text object to fit; empty text deletes the run. A character the font can't draw fails with `missing-glyph`. Some runs can't be edited yet, such as text in a Type 3 font or text written right to left: editing one fails with `content-not-editable` and the reason.
