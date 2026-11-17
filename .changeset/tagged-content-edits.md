---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

Editing a tagged PDF now keeps its tags up to date. When `doc.content.edit` deletes an object or `doc.content.editText` deletes a run of text, the content leaves the structure tree, and an element left empty leaves it too; rewritten text updates its element's `ActualText` and `Alt`. A replaced figure image loses the old alternative text. Before you save, `doc.plan().conformanceWarnings` lists the claims content edits broke: `figure-without-alt` and `content-tags-broken` on a PDF/UA-1 file, and `pdfa-font-not-embedded`, which gains a `pages` field, on a PDF/A file. The new `doc.removeConformanceClaims({ pdfa, pdfua })`, the `removeConformanceClaims` op, saves without a claim the edits broke; it is the only edit that may remove one.
