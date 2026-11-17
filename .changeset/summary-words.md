---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

A comment summary can now be written in your readers' language. Pass `words` to `doc.commentSummary`, such as `page: "Seite {label}"`. It covers the summary's title, comment count, page headings, the alternative text of the drawn pages, the status line, quotation marks, and the names of annotation kinds and review states. Any word you leave out stays in English. In Rust, `SummaryOptions::with_words` takes a `SummaryWords`.
