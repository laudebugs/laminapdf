---
title: "Recipes"
description: "Complete Lamina PDF programs, from one page of text to an archived invoice, each with its code and the PDF it writes."
version: "0.2.0-alpha.4"
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
- [Element flow](../pages/recipes/element-flow.md): a report whose parts each start below the one before, over two pages.
- [Links and outlines](../pages/recipes/links-and-outlines.md): links, an outline and page labels.

## Edit a PDF

- [Generate, then edit](../pages/recipes/generate-then-edit.md): open a file's bytes, change its metadata and save incrementally.

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
