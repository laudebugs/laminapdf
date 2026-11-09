---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Rust callers can now turn Markdown into flowing, tagged pages: `Builder::markdown(text, &MarkdownBuildOptions)` reads CommonMark with tables, footnotes, strikethrough and task lists, takes the title, language, author and subject from front matter, and lays the document out with the flow layer. Links to `#a-heading` go to that heading, and a table runs across pages with its header repeated. Images come only from the caller, through `MarkdownBuildOptions::with_image`; nothing is fetched. Raw HTML and images that weren't passed are left out with the warnings `markdown-html-ignored` and `markdown-image-missing`, which give the position in the Markdown, never its text. Without fonts, the document is set in Helvetica and Courier, which flows can now use: Latin text only, and refused under PDF/UA-1. Flows also gain code spans and code blocks, struck-out text, block quotes (`BlockQuote`), thematic breaks and lists that start at any number. The `laminapdf` crate's new `markdown` feature is on by default.
