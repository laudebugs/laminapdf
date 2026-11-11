---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Turn Markdown into a PDF from TypeScript: `builder.markdown(text, options)` turns CommonMark (with tables, footnotes, strikethrough and task lists) into tagged pages that flow, and takes the title and language from the front matter. Fonts come by role (`regular`, `bold`, `italic`, `boldItalic`, `mono`, `fallbacks`) and images by the name the Markdown uses; nothing is fetched. `options.page` sets the page size, margins, header and footer, and `contents` adds a contents page. `Font.fromCollection(bytes, index)` loads one face of a font collection (`.ttc`).
