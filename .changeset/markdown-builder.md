---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Turn Markdown into a PDF from TypeScript: `builder.markdown(text, options)` lays CommonMark with tables, footnotes, strikethrough and task lists out as tagged pages that flow, with the front matter's title and language as the document's. Fonts come by role (`regular`, `bold`, `italic`, `boldItalic`, `mono`, `fallbacks`) and images by the name the Markdown uses; nothing is fetched. `options.page` sets the page size, margins, header and footer, and `contents` adds a contents page. `Font.fromCollection(bytes, index)` loads one face of a font collection (`.ttc`). The Rust crates gain `laminapdf::markdown::image_targets`, which lists the images a Markdown file names, and `MarkdownBuildOptions::with_page_size`.
