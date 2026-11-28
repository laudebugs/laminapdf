---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Change how text a page already shows looks. `doc.content.setTextStyle(index, runs, style)` takes runs, or glyphs inside a run, and sets any of `font` (`"helvetica"`, `"times"`, `"courier"` or a font added with `doc.addBlob`, embedded as a new subset), `size`, `color`, `bold`, `italic`, `underline`, `baseline` and, for the lines of the runs' text object, `align` and `lineSpacing`. The words stay as they were, so they still copy and search as before.

`doc.content.editText` takes `fonts`: the fonts a character the run's font lacks may come from. A full copy of the run's own font adds a new subset of it; any other font substitutes. Nothing is substituted that you don't name. `doc.content.planEditText` says which characters would be drawn in which font before you commit, and the edit's `textEdits` report the same afterwards, with the fidelity and how many points the line grew.

`doc.content.setTextStyle` refuses text drawn mirrored (`content-not-editable`, reason `rotated-run`) and an `align` or `lineSpacing` on a line that starts without a line move of its own. The Rust `laminapdf-content` crate's `rewrite_segments` and `restyle_text` return `Rewritten` (the widths before and after, in text space) instead of a bare width, and `restyle_text` refuses a range across several show operators; both are new in this release.
