---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Rust callers can now pour a document into as many pages as it needs. `Builder::flow` takes headings, paragraphs of styled text and links, lists, figures, spacers, page breaks and groups kept together, with a page template of margins, a header, a footer and an optional different first page. Paragraphs split only between lines, leaving at least two lines at the foot and the head of a page. Headings stay with what follows them, and figures move whole. Headers and footers can show `{page}`, `{pages}`, `{section}` and `{title}`, so "Page 3 of 12" is right on every page. The output is tagged: each paragraph stays one element across pages, and headers and footers are marked as page furniture. New codes: `flow-content-too-large` for a line or figure taller than a page, and the warning `flow-keep-broken` for blocks kept together that can't fit on one page. The TypeScript package doesn't have flows yet.
