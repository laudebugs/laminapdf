---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

Add, change and delete links: `addLink` makes a link on an area or on lines of text, going to a page, a named destination or an `http`, `https` or `mailto` address, and `updateLink` and `deleteLink` change and remove one. `linkOnArea(page, area, target)` and `linkOnText(page, lines, target)` turn a drawn box or a selection's line boxes in page space into that edit. In a tagged document a new link joins the structure tree around the text it covers, and a file that claims PDF/UA-1 but isn't tagged refuses one (`link-not-tagged`). `pageLinks()` now gives each link's `id`, and an edit's `createdAnnotations` lists the links it added. The Rust crates have the same ops (`laminapdf::AddLink` and the rest) and methods.
