---
title: "Make and change PDFs"
description: "Turn a Markdown file into a tagged, accessible PDF with laminapdf build, and turn pages with laminapdf pages rotate."
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/cli/build-and-edit/"
---

# Make and change PDFs

Turn a Markdown file into a tagged, accessible PDF with laminapdf build, and turn pages with laminapdf pages rotate.

Make a PDF from a Markdown file, or turn pages of an existing one. Use `build` for reports, release notes and manuals you already write in Markdown; the PDF it makes flows across as many pages as the text needs, and screen readers can follow it.

## A PDF from Markdown: `build`

Take this Markdown file:

```md
---
title: Release notes
lang: en
---

# Release notes

The new version opens **twice as fast** and reads files with broken cross-references.

| Change | Pages |
| --- | --- |
| Faster open | 1 to 3 |
| Repairs | 4 |
```

`build` lays it out on A4 pages and writes the PDF:

```console
$ laminapdf build notes.md -o notes.pdf
built 1 page → notes.pdf (3 KB)
```

The PDF is tagged, so screen readers know its headings, paragraphs and tables, and its headings appear in the viewer's outline. A table's header row repeats at the top of each page it runs onto. `build` reads CommonMark with tables, footnotes, strikethrough and task lists, and `-` as the input reads Markdown from standard input.

### Title and language

The front matter's `title`, `lang`, `author` and `subject` lines set the document's metadata. When the front matter has no title or language, `--title` and `--lang` give them; the language is a tag such as `en-GB`.

### Fonts

Without fonts, `build` sets the text in Helvetica and Courier, two of the [standard 14 fonts](../../pages/reference/glossary.md) that every PDF viewer has. They cover Latin text only, and they aren't embedded in the file. Give your own fonts by role, and `build` embeds the characters the document uses:

```sh
laminapdf build notes.md -o notes.pdf \
  --font regular=NotoSans-Regular.ttf --font bold=NotoSans-Bold.ttf \
  --font fallback=NotoSansArabic-Regular.ttf
```

The roles are `regular`, `bold`, `italic`, `bold-italic` and `mono`, for code. `fallback`, which you may repeat, covers characters the others lack, such as another script. `FILE#N` takes face N of a font collection (`.ttc`). A character no font has stops the build with `missing-glyph`, which names it.

### Accessible and archival PDFs

`--profile` checks the PDF against a standard before it writes it: `pdf-ua-1` for accessibility, or `pdf-a-2b`, `pdf-a-2u` or `pdf-a-3b` for archiving. You may name PDF/UA-1 and one PDF/A profile together. Both need embedded fonts, so give fonts with `--font`; without them, the build stops with `font-not-embedded`.

```console
$ laminapdf build notes.md -o notes.pdf --profile pdf-ua-1 --font regular=NotoSans-Regular.ttf --font bold=NotoSans-Bold.ttf
built 1 page → notes.pdf (14 KB, PDF/UA-1)
```

[Accessible PDFs](../../pages/guides/accessible-pdfs.md) and [archival PDFs](../../pages/guides/archival-pdfs.md) explain what each profile asks of a document.

### Images

`build` reads images from the Markdown file's folder or the folders below it. It never fetches a remote image or reads one from outside that folder, so a Markdown file you were sent can't pull other files into the PDF. Such an image is left out with the `markdown-image-missing` warning, as raw HTML is with `markdown-html-ignored`, and the build exits 3.

### The same input gives the same bytes

The PDF has no creation date unless `--creation-date` gives one, such as `20261110`. Without a date, the same Markdown and fonts give a byte-identical PDF, so a build job can compare it with the last one.

### Page size and reports

`--page-size` takes `a4` (the default), `a0` to `a6`, `letter`, `legal` or `tabloid`. With `--json`, `build` prints a `laminapdf.build/1` report with the page count, size and profiles.

## Turn pages: `pages rotate`

`pages rotate` turns pages by a quarter turn. `+90` and `-90` turn pages from their current rotation, and `90`, `180` or `270` set it. A [page range](../../pages/cli/scripts.md#page-ranges) after a colon names the pages; without one, it turns every page:

```console
$ laminapdf pages rotate report.pdf +90:1-3 -o turned.pdf
rotated 3 pages → turned.pdf (incremental update)
```

Where the file allows it, the tool writes an incremental update, as its output says: the original bytes stay first in the new file, unchanged, and the change follows them. `--in-place` replaces the input instead of writing a new file, and needs `--force`.

Next: [use the tool in scripts and batch jobs](../../pages/cli/scripts.md).
