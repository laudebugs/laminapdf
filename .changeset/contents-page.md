---
"@laminapdf/wasm": patch
"@laminapdf/sdk": patch
---

Add a clickable contents page made from a document's bookmarks.

`doc.createContentsPage(options)` lays out a page that lists the bookmarks going to a page, one or two levels deep. Each entry shows the label its page will have once the contents page is in, and links to that page. It inserts the page where `at` says, can add a "Contents" bookmark for it, and does all of this as one undo step. In a tagged file, the page joins the structure tree as a `TOC` element. It needs the text module, and a font you pass, since Helvetica isn't embedded and a PDF/A or PDF/UA-1 file refuses it. The Rust crates have the same: `Document::create_contents_page` with `ContentsOptions`, and the builder's `Contents::listing`.
