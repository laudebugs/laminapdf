---
title: "Troubleshooting rendering"
description: "Fixes for rendering pages when PDFium isn't found, an image is too large, a page draws wrong, or laminapdf render refuses its output names."
version: "0.4.0"
url: "https://docs.laminapdf.com/troubleshooting/rendering/"
---

# Troubleshooting rendering

Fixes for rendering pages when PDFium isn't found, an image is too large, a page draws wrong, or laminapdf render refuses its output names.

These entries cover drawing pages as pixels: `laminapdf render`, and the hayro and PDFium renderers in Rust and on the web. Each entry starts from what you see, then gives the cause and the fix.

## PDFium fails to load, with `render-backend-unavailable`

**Cause:** Lamina PDF loads the native PDFium library when it runs, never when it builds, so the library must be on the machine. It looks only in the folder you name, and found no library there.

**Fix:** download PDFium for your platform from the [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases) project, and set `LAMINA_PDF_PDFIUM_DIR` to the folder that holds `libpdfium.so`, `libpdfium.dylib` or `pdfium.dll`. `laminapdf render --backend pdfium` and `PdfiumBackend::from_env()` read that variable; `PdfiumBackend::bind(dir)` takes the folder itself. See [`render-backend-unavailable`](../../errors/render-backend-unavailable.md).

## A large page or a high resolution fails with `render-too-large`

**Cause:** an image may hold at most 16,777,216 pixels (4096 × 4096) and 16,384 pixels on a side, so a hostile file can't ask for a bitmap of any size. A US Letter page goes over that size above about 420 dots per inch, so `laminapdf render --dpi 600` fails on it.

**Fix:** on the command line, lower `--dpi` or `--scale`. In code, draw the visible part of the page as tiles of 256 to 512 pixels, by giving each request a clip, instead of the whole page at once. Raise the limits only when you mean to make the larger image. See [`render-too-large`](../../errors/render-too-large.md).

## A page draws differently than in a browser

**Cause:** `laminapdf render` draws with hayro, a renderer written in Rust, unless you choose another. hayro draws most pages the same as PDFium, the renderer in Chrome, and most of the pages it gets wrong go wrong on fonts or annotations.

**Fix:** draw the page with PDFium: `laminapdf render --backend pdfium`, with PDFium installed as above. In Rust, use `laminapdf-render-pdfium`, and on the web `@laminapdf/render-pdfium`. See [How a page is drawn](../../pages/concepts/drawing-pages.md).

## `laminapdf render` exits 2: two pages would write the same file

**Cause:** `-o` takes one file-name pattern for every image, and the pattern gives two pages, or two input files, the same name. A pattern without `{n}` names every page of a document alike. `render` works out every name before it draws anything, so nothing was written.

**Fix:** put the page number in the pattern, as in `-o 'page-{n}.png'`, and the input's name too when you render several files: `-o '{stem}-{n:03}.png'`. See [Images: `render`](../../pages/cli/extract-and-render.md#images-render) and [`invalid-command-line`](../../errors/invalid-command-line.md).

## `laminapdf render` fails with `output-unwritable`

**Cause:** the folder in the `-o` pattern doesn't exist, or you can't write to it. `render` doesn't create folders.

**Fix:** make the folder first, such as `mkdir -p pages`, then run `laminapdf render in.pdf -o 'pages/{n}.png'`. See [`output-unwritable`](../../errors/output-unwritable.md).

Next: the [error and warning codes](../../errors/README.md), with what each one means and what to do.
