---
"@laminapdf/wasm": patch
"@laminapdf/sdk": patch
---

Add a clickable contents page made from a document's bookmarks.

`doc.createContentsPage(options)` lays out a page that lists the bookmarks going to a page, one or two levels deep. Each entry shows the label its page will have once the contents page is in, and links to that page. It inserts the page where `at` says, can add a "Contents" bookmark for it, and does all of this as one undo step. In a tagged file, the page joins the structure tree as a `TOC` element. It needs the text module loaded, though the document can be open in any module, and a font you pass, since Helvetica isn't embedded and a PDF/A or PDF/UA-1 file refuses it.

The Rust crates have the same as `Document::create_contents_page(&ContentsOptions, &ContentsFonts)`, and in three parts for a host whose document is open in a build without fonts: `Document::contents_listing` reads what the page lists, `ContentsListing::lay_out` lays it out with the `text` feature, and `Document::insert_contents_page` inserts it, refusing a listing the document no longer gives with `document-changed`. The builder's `Contents::listing` sets a contents block from given entries and labels.
