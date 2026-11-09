---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Rust flows can now have footnotes, a contents page and bookmarks. A footnote is a raised number in the text, numbered through the flow, with its note at the foot of the same page; a line whose notes don't fit moves to the next page with them. A contents page lists the headings with dot leaders, the page each one starts on and a link to it. The flow is laid out a second time to fill in the page numbers, and the new error code `flow-layout-unstable` reports a fault if they ever fail to settle. Every heading gets a bookmark in the document's outline, nested by level, which `Flow::with_outline(false)` turns off. Tagged documents can also use the `TOC`, `TOCI` and `Reference` structure types, which `@laminapdf/sdk` now accepts as tag names.
