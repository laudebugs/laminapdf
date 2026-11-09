---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
"@laminapdf/viewer": minor
---

Reader mode for tagged PDFs. `doc.reflow()` returns the document's blocks in its structure tree's order: headings, paragraphs, list items, quotes, captions and notes as text, and each figure, formula and table as a crop of its page with its alternate text, never as reflowed text. It checks the tree first: a file whose tree it can't trust comes back with `available: false` and the reason. It needs the extract module. The viewer's worker serves it as `client.reflow()`.
