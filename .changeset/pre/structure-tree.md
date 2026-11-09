---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

`doc.structureTree()` reads a tagged document's structure tree: its elements in reading order, down to the marked content each owns on a page. Each element gives its standard type (`H1`, `P`, `LI`, `Figure` …), alternate text, language and actual text; custom types resolve through the role map. A text run's `mcid` links it to its element, so a viewer can give assistive technology a document's headings, paragraphs, lists and tables.

`doc.isTagged` says whether the document claims to be tagged. Both need the extract or text module, as `page.text()` does. For Rust callers, `Document::structure_tree()` returns `StructureNode`s.
