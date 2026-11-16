---
"@laminapdf/wasm": patch
"@laminapdf/sdk": patch
---

Type on a page: `doc.textBoxAt` and `doc.calloutAt` give the edit that adds a text box, or a callout with a leader line, from a box and points in page space. The text can be plain or rich, with bold and italic words, and Lamina PDF draws the box into the file, upright as the page is shown. Helvetica, Times and Courier work in the extract and text modules. A font given by its bytes (`addBlob`) and fallback fonts for characters it lacks, such as Chinese or Arabic, need the text module and are embedded as subsets. `updateAnnot` changes a box's text, font or look, and warns with `rich-text-simplified` when a box made by another app loses styles the subset doesn't keep. In Rust, the same edits are `Document::text_box_at` and `callout_at`.
