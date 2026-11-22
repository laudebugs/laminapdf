---
title: "Inspect a PDF"
description: "Read the version and the standards a PDF claims, its fonts and colour profiles, and any object, stream or reference inside it, without changing it."
version: "0.7.0"
url: "https://docs.laminapdf.com/guides/inspect-a-pdf/"
---

# Inspect a PDF

Read the version and the standards a PDF claims, its fonts and colour profiles, and any object, stream or reference inside it, without changing it.

You'll look inside a PDF: its version, the standards it claims, its fonts and colour profiles, and the objects it is made of. You need this to debug a file another tool wrote, to check a font is embedded before printing, or to build a developer view.

Nothing here changes the document, and nothing is verified: a claim is what the file says. In TypeScript these reads need the extract or text module (`loadExtract()`), as `properties()` does.

## Version and claims

`conformance()` gives the `%PDF-x.y` header's version, the catalog's `/Version`, and the claims the file makes: PDF/A (part and level, from its XMP metadata), PDF/UA (part) and PDF/X (`GTS_PDFXVersion`, from the XMP metadata or the Info dictionary). A claim says the file was written to meet a standard; [veraPDF](https://verapdf.org/) tells you whether it does.

## Fonts and colour profiles

`fonts(pages)` lists the fonts the pages use, once each: the pages' own, those of the forms and patterns they draw, and those of their annotations' appearances, as `pdffonts` lists them. `properties()` lists only the fonts the page content uses, without coverage. Each font says its type, whether the file embeds it or only a subset of it, its encoding, its object, and whether a [ToUnicode](../../pages/reference/glossary.md#tounicode) map turns its codes into text. `toUnicodeCoverage` counts how many of the font's codes that map covers: a font with codes the map leaves out gives text that can't be searched or copied.

`colorProfiles()` lists the output intents (what a PDF/A or PDF/X file is meant to print or show on) and every ICC profile the file carries, each with what its header says: its colour space, device class, version and description.

## Objects, streams and references

- `objectCount()` counts the objects without reading any, and `objectEntry(num)` reads one object's entry.
- `objects()` lists every object by number, with its kind, its `/Type` and `/Subtype`, and where it is stored: at an offset in the file, inside an [object stream](../../pages/reference/glossary.md#object-stream), or made in this session.
- `trailer()` is the root of it all: `/Root`, `/Info`, `/ID`.
- `object(num)` gives one object's value as plain data (dictionaries, arrays, names, strings, references), a stream's filters and stored length, and the revision that last wrote it. `objectSyntax(num)` writes it as PDF syntax, to copy.
- `streamBytes(num, view, max)` reads the first `max` bytes of a stream, `"raw"` as stored or `"decoded"`. A decoded stream is decoded only that far, and one byte more to know whether there is more: `total` is absent for a longer one. Decoding never goes past the document's [limits](../../pages/guides/untrusted-files.md): with `max` past `maxStreamDecoded`, a stream that inflates further is refused with [`limit-exceeded`](../../errors/limit-exceeded.md), and its raw bytes can still be read.
- `referencesTo(num)` lists the places that refer to an object, with the key path inside each (`/Kids[2]`). The first call reads every object to index them, so a large file takes a moment; later calls reuse the index until the document changes.
- `revisionCount()` counts the revisions; `revisions()` lists each revision an incremental update added, with the objects it added, changed and freed, and whether a signature covers exactly that revision.

These reads are the same in Rust (`Document::conformance`, `fonts`, `color_profiles`, `objects`, `object`, `stream_bytes`, `references_to`, `object_syntax`, `revisions`) and in TypeScript, where names are camelCase.

## In the reader

The web reader shows all of this in its object inspector: More › Inspect objects, Document properties › Inspect…, or the commands Inspect objects and Go to object… in the command palette. It replaces the pages until Back to pages, with tabs for Objects, Revisions, Fonts, Colour and Conformance. It isn't offered on a phone-sized screen.

Next: [Check what a file hides](../../pages/guides/check-what-a-file-hides.md), for a file's earlier revisions and hidden content.
