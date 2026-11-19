---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

You can now move comments between Lamina PDF and other PDF apps. `doc.exportAnnotations({ format: "xfdf" })` (`export_annotations` in Rust) writes the document's comments as XFDF, which Acrobat and most PDF apps import. `"fdf"` writes FDF instead, and `"markdown"` writes Markdown for a notes app, each highlight's text as a quote. XFDF and FDF keep every reply with its comment, carry attached files, and record which PDF they came from. Exporting changes nothing in the document.

`doc.importAnnotations(bytes)` (`import_annotations` in Rust) brings an XFDF or FDF file's comments in as one change, so a single undo takes them all back. By default it keeps the comments already in the document, finds those the file names again, and adds the rest; `{ mode: "replace" }` replaces the comments on the pages the file covers, and `{ mode: "add" }` adds everything as copies. Each comment gets an appearance and keeps the pop-up the file places. `doc.previewImport(bytes)` says what an import would do first: how many comments it adds and where, what it leaves out, such as a sound or a form field's value, and whether the file was made for another PDF.

A file that can't be read fails with `interchange-malformed`, naming the line where reading stopped. An XFDF file that declares a DTD is refused with `interchange-dtd`, since a DTD can define entities that expand without end or read other files.

In Rust, `AnnotCommon::popup` places a comment's pop-up, and `AnnotSpec::from_entries` and `AnnotCommon::from_entries` read what an annotation dictionary describes.
