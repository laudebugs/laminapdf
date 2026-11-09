---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
"@laminapdf/viewer": minor
---

Links in PDFs now work in the viewer. `doc.pageLinks(index)` lists a page's links in page space, each with what following it does: a place in the document, a web address, another file, a named action, or an action such as JavaScript, which Lamina PDF never runs. The viewer draws an anchor over each link. Links inside the document go there, web links ask the app to confirm first, and anything else does nothing. `viewer.history` records each jump, so Back returns to where the reader was, and the web reader keeps those jumps in the browser's history.
