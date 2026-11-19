---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
---

You can now move comments between Lamina PDF and other PDF apps. `doc.exportAnnotations({ format: "xfdf" })` (`export_annotations` in Rust) writes the document's comments as XFDF, which Acrobat and most PDF apps import. `"fdf"` writes FDF instead, and `"markdown"` writes Markdown for a notes app, each highlight's text as a quote. XFDF and FDF keep every reply with its comment, carry attached files, and record which PDF they were made for. Exporting changes nothing in the document.

`doc.importAnnotations(bytes)` (`import_annotations` in Rust) brings an XFDF or FDF file's comments in as one change, so a single undo takes them all back. By default it leaves the comments the file names (by `/NM`) as they are and adds the file's other comments; `{ mode: "replace" }` replaces each comment the file names with the file's version, and `{ mode: "add" }` adds the file's comments as copies. Each comment gets an appearance and keeps the pop-up the file places. `doc.previewImport(bytes)` says what an import would do first: how many comments it adds and where, what it leaves out, such as a sound or a form field's value, and whether the file was made for another PDF.

A file that can't be read fails with `interchange-malformed`, naming the line where reading stopped. An XFDF file that declares a DTD is refused with `interchange-dtd`, since a DTD can define entities that expand without end or read other files.

In Rust, `AnnotCommon::popup` places a comment's pop-up, and `AnnotSpec::from_entries` and `AnnotCommon::from_entries` read what an annotation dictionary describes.
