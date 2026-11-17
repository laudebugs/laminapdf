---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

A comment summary can now be written in your readers' language. Pass `words` to `doc.commentSummary` with the summary's own words, such as `page: "Seite {label}"`: its title, the line counting the comments, each page's heading, the drawn page's description, the status line, and the names of annotation kinds and review states. Any word you leave out stays in English. In Rust, `SummaryOptions::with_words` takes a `SummaryWords`.
