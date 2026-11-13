---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

Highlight, underline, strike out and squiggle text, leave sticky notes and mark insertions with carets.

`addAnnot` makes each mark with an appearance Lamina PDF draws itself, so every reader that draws stored appearances shows it the same. `markText(page, kind, lines)`, `noteAt(page, at, comment, icon)` and `caretAt(page, area)` turn a selection's line boxes, a click or a box in page space into that edit, and `replaceText(page, lines, replacement)` proposes a replacement as a strike-out and a caret grouped in one undo step. In a tagged document a mark joins the structure tree after its text, and its description is the text it marks. `updateAnnot` now changes a mark's `color`, `opacity` and geometry (`spec`), drawing it again. Saving warns when a mark breaks a PDF/A or PDF/UA claim: `annot-without-contents`, `annot-not-tagged`, `pdfa-annot-flags` and `pdfa-transparency`. The Rust crates have the same ops (`laminapdf::AddAnnot` and the rest) and methods; `laminapdf-build`'s `AnnotationAppearances` draws the appearances for code that uses `laminapdf-ops` directly.
