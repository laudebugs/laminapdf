---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
"@laminapdf/viewer": minor
---

Read a text box back and change what it holds. `doc.textBox(id)` returns any app's text box in the same form you write one in: its text with bold and italic, its font and styling, and where it sits on the page. Change it with a `freeText` patch to `updateAnnot`. When another app's box uses styles Lamina PDF doesn't keep, such as a coloured word, `simplified` is true, and changing the box writes it again without them.

In `@laminapdf/viewer`, `client.textBox(id)` reads a box in the worker. A document that opened before the extract module loaded can't take text boxes: adding one fails with `appearance-needs-text`. Call `client.editInTextCopy()` first, or `client.editInTextCopy({ text: true })` for a font given by its bytes or fallback fonts. Undo and redo still reach every edit made before the call. To check first, `doc.hasText` says whether a document can lay out text in a font given by its bytes.
