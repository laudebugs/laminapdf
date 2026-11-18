---
title: "Attachments"
description: "List the files a PDF carries, save one out with its checksum checked, and attach, describe or delete files, PDF/A and signed files included."
version: "0.6.0"
url: "https://docs.laminapdf.com/guides/attachments/"
---

# Attachments

List the files a PDF carries, save one out with its checksum checked, and attach, describe or delete files, PDF/A and signed files included.

You'll work with the files a PDF carries: a spreadsheet behind a chart, an e-invoice's XML, a contract's earlier draft. You need this to give people those files, or to attach files to a document your app makes or edits.

[Make an invoice Factur-X](../../pages/recipes/attach-factur-x.md) shows every step in Rust and TypeScript. This page explains the parts.

## List the files

`attachments()` lists every file the document carries, each once. A file says what holds it (`holders`):

- the document's list of attachments, which every viewer shows;
- the document, a page or an annotation it is associated with, for example the data a chart on page 3 was made from;
- a paperclip annotation on a page.

Each file gives its name, description, MIME type, size and dates as the PDF states them, and its relationship to what it belongs to: `source`, `data`, `alternative`, `supplement` or another.

## Save a file out

`attachmentBytes(id)` gives a file's bytes exactly as attached, and compares them with the MD5 checksum the PDF records: `matches`, `mismatch` or `absent`. A mismatch means the bytes changed after the checksum was written, so say so before anyone trusts the file.

Lamina PDF never opens an attached file. Saving one is your app's job, and a program, such as an `.exe` or a script, deserves a warning first.

The document's [limits](../../pages/guides/untrusted-files.md) apply: a file larger than `maxStreamDecoded` is refused with [`limit-exceeded`](../../errors/limit-exceeded.md) before it is read.

## Attach, describe and delete

Each of these is one edit, which undo takes back:

- `attachFile(name, bytes, mime, options)` attaches a file. A file larger than `attachmentLimit()`, the document's `maxStreamDecoded`, is refused with [`attachment-too-large`](../../errors/attachment-too-large.md): it could never be saved out again. `options.attachTo` associates it with the document, a page or an annotation, and `options.relationship` says how. The dates come from your app's clock, never from Lamina PDF's.
- `updateAttachment(id, patch)` changes a file's name, description, MIME type or relationship. Its bytes stay as they are.
- `deleteAttachment(id)` takes a file out of the list and every association. A paperclip annotation that holds the same file keeps it, and the edit warns [`attachment-still-held`](../../errors/attachment-still-held.md).

An [incremental save](../../pages/reference/glossary.md#incremental-save) keeps a deleted file in the PDF's earlier revisions; save with `mode: "fullRewrite"` to drop it.

## PDF/A and signed files

- **PDF/A-1 and PDF/A-2** allow no attached files but PDF/A ones, so attaching warns [`pdfa-attachment-forbidden`](../../errors/pdfa-attachment-forbidden.md) in `plan()`.
- **PDF/A-3** allows any file that has an association, a MIME type and a modification date; one without warns [`pdfa-attachment-incomplete`](../../errors/pdfa-attachment-incomplete.md). Attaching never raises the file's PDF version.
- **A certified file** allows no attachment edits at any certification level. A file with approval signatures only takes them in an update after the signatures.
- **A file with usage rights** takes each edit only when its rights grant it: attaching needs the right to create files, changing one the right to modify, deleting one the right to delete. Attaching to a page is never covered, and attaching to an annotation also needs the right to modify annotations. `wouldViolate(op)` says so before the edit.

## Read a portfolio

A portfolio is a PDF that presents its attached files as the main content, with columns such as an owner or a due date. `collection()` reads its columns, how its files are sorted, the view it asks for and each file's values; it is `undefined` for any other PDF.

Next: [Save a PDF](../../pages/guides/save-a-pdf.md), for what a signed or repaired file allows.
