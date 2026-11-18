---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
"@laminapdf/viewer": minor
---

Read a text box back and change what it holds. `doc.textBox(id)` returns a text box as its spec, whichever app made it: its text with bold and italic, font, size, alignment, fill, border and kind, plus where it sits on the page. You then change it with a `freeText` patch to `updateAnnot`. When another app's box uses styles Lamina PDF doesn't keep, such as a coloured word, `simplified` is true, and changing the box writes it again without them. `doc.hasText` says whether a document can lay out text in a font given by its bytes.

In `@laminapdf/viewer`, `client.textBox(id)` reads a box in the worker, and `client.editInTextCopy({ text })` lets a document that opened before the extract module loaded take text boxes. Before, the base module refused them with `appearance-needs-text`. The worker moves the document into a copy that lays out text, in the text module when `text` is set, and the edit history comes with it, so undo and redo still reach every edit made before the switch.
