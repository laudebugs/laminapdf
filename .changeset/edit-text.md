---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

You can now change the text a page already shows. `doc.content.textRuns(index)` (`text_runs` in Rust) lists a page's runs of text in reading order, each with its text, box, font, size and line, and a reason when it can't be edited. The new `editText` op (`doc.content.editText(index, run, text)`) gives a run new text in the font it is set in, and moves the rest of the line to fit; empty text deletes the run. A character the font can't draw fails with `missing-glyph`, and a run in a Type 3 font, in vertical writing, or with glyphs turned or out of order fails with `content-not-editable` and its reason.
