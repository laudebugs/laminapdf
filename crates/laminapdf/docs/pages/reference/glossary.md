---
title: "Glossary"
description: "The PDF and Lamina PDF terms the docs use, each in a sentence or two."
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/glossary/"
---

# Glossary

The PDF and Lamina PDF terms the docs use, each in a sentence or two.

The terms the docs use, in plain words. Each page links a term here the first time it uses one.

## How Lamina PDF works

### Overlay

Where Lamina PDF keeps your edits. The bytes you opened never change; the overlay sits on top of them, and saving writes it out. See [how Lamina PDF thinks about a PDF](../../pages/concepts/how-lamina-thinks.md).

### Incremental save

A save that keeps the original bytes as they are and appends a new revision with your edits after them. Signatures over the original stay valid.

### Full rewrite

A save that writes the whole file again. The result is smaller and tidier, but it's no longer the original bytes, so it breaks any signature.

### Façade

The `laminapdf` crate: one API in front of Lamina PDF's core crates, which most Rust code depends on alone.

### Glue

The JavaScript that sits next to each `.wasm` file. It loads the module and converts values between JavaScript and Rust.

### Export condition

A key in a package's `exports` map, such as `node`, `browser`, `deno` or `workerd`, that tells a runtime or bundler which file to load. `@laminapdf/sdk` has one per runtime.

## Inside a PDF file

### COS

PDF's object layer: the dictionaries, arrays, numbers, strings, names, streams and references a file is made of.

### Content stream

A stream of drawing instructions for a page: where to put text, which lines and shapes to draw, which images to place.

### XObject

A reusable piece of content that a content stream draws by name: an image, or a form, which is a group of drawing instructions.

### Cross-reference table

The index at the end of a file that gives the byte offset of every object, so a reader can jump straight to one. Newer files store it as a compressed stream instead.

### Xref

Short for [cross-reference table](#cross-reference-table).

### Object stream

A compressed stream that holds many small objects together, to make the file smaller. Files from PDF 1.5 on can use them.

### Page space

Lamina PDF's coordinates: points from the top-left corner of the page as it's displayed, with y going down. See [pages and coordinates](../../pages/concepts/pages-and-coordinates.md).

### Crop box

The part of a page a viewer shows; anything outside it is cut off. Page space starts at its top-left corner.

### User unit

A page entry that makes each user-space unit bigger than 1/72 of an inch, for very large pages. Lamina PDF applies it, so page-space numbers are always points of 1/72 inch.

### User space

PDF's own coordinates: points from the bottom-left corner of the page, with y going up, before any rotation. Lamina PDF's API uses page space instead; see [pages and coordinates](../../pages/concepts/pages-and-coordinates.md).

## Fonts and text

### Standard 14 fonts

The fourteen fonts every PDF reader supplies: Times, Helvetica and Courier, each in four styles, plus Symbol and ZapfDingbats. A PDF can use them without embedding them.

### Type 0

A font type whose character codes can be more than one byte long, so a font can reach any of its glyphs. Lamina PDF embeds every font except the standard 14 as Type 0.

### CMap

A table that maps a font's character codes to character IDs (CIDs), which pick its glyphs, or, in a ToUnicode CMap, to Unicode text.

### ToUnicode

A CMap a font carries that maps its codes to Unicode, so a reader can copy, search and extract the text.

## Signatures

### DocMDP

The permissions of a certifying signature: which changes the signed document still allows, such as none, filling in forms, or comments. DocMDP stands for document modification detection and prevention.

### UR3

A usage-rights signature, which unlocks features such as saving form data in Adobe Reader. An edit it doesn't allow invalidates it.
