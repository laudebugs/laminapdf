---
title: "Recipes"
description: "Complete Lamina PDF programs, from one page of text to an archived invoice, each with its code and the PDF it writes."
version: "0.3.0"
url: "https://docs.laminapdf.com/recipes/"
---

# Recipes

Complete Lamina PDF programs, from one page of text to an archived invoice, each with its code and the PDF it writes.

Each recipe is a complete program to copy: what it makes, the code, how it works and what to change. Pick the one closest to the document you need, and change it from there.

Most recipes exist twice, in Rust and in TypeScript, and both write exactly the same bytes. Lamina PDF's tests check that on every change, in Rust and in every JavaScript runtime: Node, Chromium, Firefox, WebKit, Deno, Bun and workerd. Each recipe shows the tested code, and its reference PDF is the file that code writes. To test your own PDFs the same way, see [Test the PDFs you generate](../pages/guides/test-generated-pdfs.md).

To run a Rust recipe from a checkout of Lamina PDF, use `cargo run -p laminapdf --example <name>`. Its header comment lists the arguments it takes, such as the font or image files it reads.

## Create a PDF

- [Hello, world](../pages/recipes/hello.md): one page of text in a standard 14 font.
- [An invoice](../pages/recipes/invoice.md): an embedded font, a table of line items, and tags for screen readers.
- [Multilingual text](../pages/recipes/multilingual.md): five scripts, each in its own embedded font.
- [Images and vectors](../pages/recipes/images-and-vectors.md): paths, colour, transparency, a clip and two images.
- [Turn photos into a PDF](../pages/recipes/images-to-pdf.md): one page per photo, fitted on A4, each JPEG embedded as it is.
- [Element flow](../pages/recipes/element-flow.md): a report whose parts each start below the one before, over two pages.
- [Turn Markdown into a PDF](../pages/recipes/markdown-to-pdf.md): a report written in Markdown, from the command line, TypeScript or Rust.
- [Fill an invoice from JSON](../pages/recipes/invoice-from-json.md): a JSON template filled from data, its line items repeated into a table across pages.
- [Links and outlines](../pages/recipes/links-and-outlines.md): links, an outline and page labels.

## Edit a PDF

- [Generate, then edit](../pages/recipes/generate-then-edit.md): open a file's bytes, change its metadata and save incrementally.
- [Edit the metadata](../pages/recipes/edit-metadata.md): a title, authors and keywords in both copies a PDF keeps, checked before the save.
- [Choose how a PDF opens](../pages/recipes/set-initial-view.md): the page and zoom, the panel beside the page and the title in the window.
- [Edit the bookmarks](../pages/recipes/edit-bookmarks.md): add, move and style bookmarks, and name places links jump to.
- [Record your own metadata](../pages/recipes/edit-xmp.md): properties of your own in the XMP metadata, in a PDF/A file that stays valid.
- [Organise pages](../pages/recipes/organise-pages.md): delete, move, insert and rotate pages in one edit, with the bookmarks and links that pointed at a deleted page sent on.
- [Duplicate pages](../pages/recipes/duplicate-pages.md): copy a page so the copy has form fields, annotations and tags of its own.
- [Copy pages between PDFs](../pages/recipes/copy-pages.md): copy pages in from another file, replace a page, or make a new PDF from some pages.
- [Merge and split PDFs](../pages/recipes/merge-and-split.md): merge files into one with a bookmark for each, or split a file by chapter.

## Read a PDF

- [Extract text for an LLM](../pages/recipes/extract-markdown.md): Markdown, JSON with boxes, and chunks for retrieval, in TypeScript and Rust.

## Meet a standard

- [A PDF/UA report](../pages/recipes/pdf-ua-report.md): an accessible report a screen reader can follow.
- [A PDF/A archive](../pages/recipes/pdf-a-archive.md): a letter for long-term archiving.
- [A PDF/A-3 invoice with XML](../pages/recipes/pdf-a3-invoice-with-xml.md): an invoice with its data attached.

## Run it in production (TypeScript)

- [A 100-page report](../pages/recipes/report.md): the case Lamina PDF times in every runtime.
- [Open someone else's file](../pages/recipes/open-untrusted.md): a password and tighter limits, with the outcome as a value.
- [Save in a Web Worker](../pages/recipes/web-worker-client.md): progress and cancelling, off the main thread.
- [A Cloudflare Worker](../pages/recipes/cloudflare-worker.md): a PDF per request.

Next: [Hello, world](../pages/recipes/hello.md), the smallest PDF Lamina PDF writes.
