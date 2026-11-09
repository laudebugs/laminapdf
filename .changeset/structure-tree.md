---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

A tagged document's structure tree is readable: `doc.structureTree()` gives its elements in reading order, each with its standard type (`H1`, `P`, `LI`, `Figure` …, custom types followed through the role map), alternate text, language and actual text, down to the marked content each owns on a page. A text run's `mcid` links it to its element, so a viewer can give assistive technology a document's headings, paragraphs, lists and tables. `doc.isTagged` says whether the document claims to be tagged. Both need the extract or text module, as `page.text()` does. For Rust callers, `Document::structure_tree()` returns `StructureNode`s.
