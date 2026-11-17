---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

Editing a tagged PDF now keeps its tags up to date. When `doc.content.edit` deletes an object or `doc.content.editText` deletes a run of text, the content leaves the structure tree, and an element left empty leaves it too; rewritten text updates its element's `ActualText` and `Alt` when they hold the old words. A replaced figure image loses the old alternative text. Before you save, `doc.plan().conformanceWarnings` lists the claims content edits broke: `figure-without-alt` and `content-tags-broken` on a PDF/UA file, and `pdfa-font-not-embedded`, which gains a `pages` field, on a PDF/A file (once edits can add fonts). The new `doc.removeConformanceClaims({ pdfa, pdfua })` (op `removeConformanceClaims`) takes the PDF/A or PDF/UA claim out of the XMP metadata. It is the only edit that may.
