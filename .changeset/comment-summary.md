---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

You can now make a summary of a document's comments. `doc.commentSummary({ font })` returns a new PDF that lists every comment with its number, author, date, status, the text it marks, its note and its replies. By default each page is drawn small beside its comments, with their numbers marked on it; the drawing is a copy of the page, so its text stays text. `layout: "list"` leaves the pages out. The summary is tagged and conforms to PDF/UA-1, and the reviewed document doesn't change. A page set in a font the file doesn't embed is listed without its drawing, with the `summary-page-not-drawn` warning. In Rust, `Document::comment_summary` takes `SummaryOptions`, and `laminapdf-build` gains `Builder::page_forms`, which copies pages of another document as Form XObjects, and `FlowBlock::Beside`, which sets blocks beside a figure.
