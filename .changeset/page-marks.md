---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Put a watermark, a background, a header and footer, or Bates numbers on many pages at once, then change or remove them, even after the file is saved and reopened. A mark is part of each page: it prints and every reader draws it, and in a tagged file it is an artifact that screen readers skip. `doc.content.addWatermark(pages, mark)` and `addBackground` show text or an image centred on each page, turned by `angle` and faded by `opacity`; with `hideable: true` a reader's layers panel can switch the mark off. `setHeaderFooter` takes six slots of text, three across the top and three across the bottom, with the fields `{page}`, `{pages}`, `{label}`, `{date}` and `{file}`, filled when the edit is applied. `applyBates` numbers pages from `start` with a prefix, a suffix and zero padding. `pageMarks(index)` lists a page's marks, `updatePageMark` replaces one and `deletePageMark` takes it off, by the ID the mark has on every page.

One new code says what went wrong: `unknown-page-mark`, a page that draws no mark with the ID. The rest are the content ops' own, such as `content-id-in-use` and `content-glyph-missing`.
