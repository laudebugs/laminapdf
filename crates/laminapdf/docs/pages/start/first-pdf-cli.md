---
title: "Your first PDF on the command line"
description: "Install the laminapdf command, turn a Markdown file into a PDF, then inspect it and read it back."
version: "0.4.0"
url: "https://docs.laminapdf.com/start/first-pdf-cli/"
---

# Your first PDF on the command line

Install the laminapdf command, turn a Markdown file into a PDF, then inspect it and read it back.

You'll turn a short Markdown file into a tagged PDF with the `laminapdf` command, then check the PDF, read its text back and draw its page as an image. It takes about five minutes, and you need no Node or Rust project.

## Install the tool

On macOS and Linux, install it with Homebrew; anywhere npm runs, install it with npm:

```sh
brew install laudebugs/tap/laminapdf
```

```sh
npm install -g @laminapdf/cli@0.4.0
```

[Install the tool](../../pages/cli/install.md) has downloads for every system and how to check them. Check that it runs:

```console
$ laminapdf --version
laminapdf 0.4.0
```

## Make a PDF

Save this Markdown as `notes.md`:

```md title="notes.md"
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

Then build it. `build` lays it out on A4 pages:

```console
$ laminapdf build notes.md -o notes.pdf
built 1 page → notes.pdf (3 KB)
```

## Look at what you made

`info` describes the PDF, and `check` reads all of it and reports anything wrong. The front matter's title became the document's title, and the PDF is tagged, so screen readers can follow its headings and table:

```console
$ laminapdf info notes.pdf
notes.pdf  PDF 1.7 · 1 page · 3 KB · not encrypted · tagged
Title: Release notes
Pages: 595.28 × 841.89 pt
$ laminapdf check notes.pdf
notes.pdf: ok · 1 page · 33 objects
```

## Read it back

`extract` reads the text back in reading order, here as Markdown, with the table as a table:

```console
$ laminapdf extract notes.pdf --format md
# Release notes

The new version opens twice as fast and reads files with broken cross-references.

| Change | Pages |
| --- | --- |
| Faster open | 1 to 3 |
| Repairs | 4 |
```

## See the page

`render` draws the page as a PNG image, at 150 dots per inch. Open `notes.png`, or `notes.pdf` in any PDF viewer:

```console
$ laminapdf render notes.pdf -o notes.png
page 1: 1240 × 1753 px → notes.png
```

## What happened

`build` read the Markdown and set it in Helvetica, one of the [standard 14 fonts](../../pages/reference/glossary.md) that PDF viewers have; code would use Courier. It wrote a tagged PDF with no creation date, so the same file always gives the same bytes. `info`, `check`, `extract` and `render` each opened that PDF and printed what they found.

Next, see what else the tool does in [the command line](../../pages/cli.md), or make the same PDF [with your own fonts and an accessibility profile](../../pages/cli/build-and-edit.md#accessible-and-archival-pdfs).
