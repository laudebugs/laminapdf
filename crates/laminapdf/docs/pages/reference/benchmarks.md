---
title: "Benchmarks"
description: "How fast Lamina PDF makes PDFs in each runtime, how large its WebAssembly files are, and how well it extracts text on a public benchmark."
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/benchmarks/"
---

# Benchmarks

How fast Lamina PDF makes PDFs in each runtime, how large its WebAssembly files are, and how well it extracts text on a public benchmark.

This page gives Lamina PDF's measured numbers: how long it takes to make a PDF in each runtime, how much your app downloads, and how well it extracts text from a public benchmark of real pages. Each table states the version, the date and the machine it came from, so you can judge how the numbers apply to yours.

## How long a PDF takes to make

Each figure is the time to build a document and save it to bytes, after the WebAssembly module has loaded. Every runtime builds the same three documents and gets the same bytes:

- **First PDF:** the first one-page "Hello" PDF after `init()`, before the runtime has optimised anything.
- **Hello:** the same PDF afterwards, the median of 20 builds.
- **One-page invoice:** the [invoice recipe](../../pages/recipes/invoice.md), a tagged A4 page with an embedded font and a table of items, the median of 20 builds.
- **100-page report:** the [report recipe](../../pages/recipes/report.md), with two embedded fonts and a table on every page, the median of 3 builds.

| Runtime | Version | First PDF | Hello | One-page invoice | 100-page report | WebAssembly memory |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Node 24 | 24.19.0 | 19.8 ms | 0.8 ms | 3.5 ms | 222 ms | 4.3 MB |
| Deno | 2.9.6 | 23.8 ms | 0.7 ms | 3.6 ms | 238 ms | 4.3 MB |
| Bun | 1.4.4 | 14.9 ms | 1.4 ms | 3.6 ms | 151 ms | 4.3 MB |
| Cloudflare Workers (workerd) | 4.152.0 | 25.0 ms | 2.0 ms | 5.0 ms | 226 ms | 4.3 MB |
| Chromium | 1.64.1 | 18.3 ms | 0.7 ms | 3.7 ms | 215 ms | 4.3 MB |
| Firefox | 1.64.1 | 2.0 ms | 1.0 ms | 5.0 ms | 313 ms | 4.3 MB |
| WebKit (Safari's engine) | 1.64.1 | 28.0 ms | 2.5 ms | 4.0 ms | 169 ms | 4.3 MB |
| Rust, native | 1.99.1 | – | – | 0.9 ms | 45.9 ms | – |

Measured with Lamina PDF 0.2.0-alpha.3 (commit `40c4179`) on 8 November 2026, on Apple M1 Pro (8 cores, 16 GB of memory), macOS 27.0.1, with a load average of 8.2 when the run began.

The browsers run in Playwright at the version shown, and Cloudflare Workers run in workerd through wrangler. WebAssembly memory is the modules' memory after the reports, in megabytes of 1,000,000 bytes. Because WebAssembly memory never shrinks, the figure is also their peak. The JavaScript heap isn't included. The native Rust row is the same invoice and report built by the `laminapdf` crate, the median build as measured by criterion, a Rust benchmarking library.

## How much your app downloads

`@laminapdf/sdk` loads one WebAssembly file, chosen by what you use. `base` reads, edits and saves documents and draws shapes and images; `text` adds fonts and text; `extract` adds text extraction without the fonts. Each file has a size budget, and a change that grows a file past its budget can't be merged until the budget is raised on purpose.

| File | Gzip | Brotli | Budget (gzip) |
| --- | ---: | ---: | ---: |
| `lamina_pdf_base.wasm` | 400.0 kB | 318.2 kB | 410.0 kB |
| `lamina_pdf_extract.wasm` | 668.1 kB | 519.0 kB | 690.0 kB |
| `lamina_pdf_pdfium.wasm` | 2,158.0 kB | 1,653.6 kB | 2,243.2 kB |
| `lamina_pdf_render_hayro.wasm` | 1,781.7 kB | 1,392.4 kB | 1,855.0 kB |
| `lamina_pdf_text.wasm` | 1,153.0 kB | 875.5 kB | 1,200.0 kB |
| `@laminapdf/sdk` (JavaScript) | 19.2 kB | – | 20.0 kB |
| `@laminapdf/sdk/worker` (JavaScript) | 4.7 kB | – | 5.0 kB |
| `@laminapdf/sdk/worker/entry` (JavaScript) | 19.2 kB | – | 20.0 kB |

Measured with Lamina PDF 0.2.0-alpha.3 (commit `40c4179`) on 8 November 2026, on Apple M1 Pro (8 cores, 16 GB of memory), macOS 27.0.1, with a load average of 8.2 when the run began.

Sizes are in kilobytes of 1,000 bytes, as a server sends them compressed. The JavaScript rows are `@laminapdf/sdk`'s code, minified: its API with the code that loads the WebAssembly, then the Web Worker client and the file that runs in the Web Worker.

## How well it extracts text

[olmOCR-bench](https://huggingface.co/datasets/allenai/olmOCR-bench), by the Allen Institute for AI, checks extracted text with small tests. A test checks that one passage comes before another, that a table cell sits beside its neighbours, or that a page's running header is left out. For example, on a two-column paper, a test passes when the end of the left column comes before the top of the right one. These figures use the three subsets made of pages that store their text, not scans. The benchmark's own tests score the Markdown that Lamina PDF writes.

| Subset | What it checks | Pages | Tests | Passed | Score |
| --- | --- | ---: | ---: | ---: | ---: |
| `multi_column` | reading order: 884 tests that one passage comes before another, on 231 multi-column pages | 231 | 884 | 461 | 52.1 % |
| `table_tests` | tables: 1,020 tests that a cell's neighbours (up, down, left, right, headings) are right, and 2 baseline tests, on 188 pages | 188 | 1,022 | 245 | 24.0 % |
| `headers_footers` | running headers, footers and page numbers left out: 753 tests, and 7 baseline tests, on 266 pages | 266 | 760 | 457 | 60.1 % |
| Baseline | the page gave text, without long repeats (one test per page) | – | 676 | 578 | 85.5 % |

Overall, as the benchmark averages its groups, 55.4 %; without the baseline group, 45.4 %. 672 pages were extracted and 13 failed. On one thread that is 428 pages a second, with a median of 2.0 ms a page.

Measured with Lamina PDF 0.2.0-alpha.3 (commit `40c4179`) on 8 November 2026, on Apple M1 Pro (8 cores, 16 GB of memory), macOS 27.0.1, with a load average of 8.7 when the run began.

Every page in the benchmark is a file of its own, so the time per page includes opening the file. A page with no text, such as a scan, counts against the score, as it would for you. Lamina PDF reads one page at a time on one thread here, without a layout model or OCR.

### Also looking for tables without ruled lines

Most tables in the benchmark have no ruled lines. Extraction finds tables like these when you ask it to look for text lined up in columns too: `tables: "lattice-and-stream"` in TypeScript, `TableMode::LatticeAndStream` in Rust. It finds more tables, but it reads some multi-column pages as tables, which costs those pages their reading order.

| Subset | What it checks | Pages | Tests | Passed | Score |
| --- | --- | ---: | ---: | ---: | ---: |
| `multi_column` | reading order: 884 tests that one passage comes before another, on 231 multi-column pages | 231 | 884 | 131 | 14.8 % |
| `table_tests` | tables: 1,020 tests that a cell's neighbours (up, down, left, right, headings) are right, and 2 baseline tests, on 188 pages | 188 | 1,022 | 480 | 47.0 % |
| `headers_footers` | running headers, footers and page numbers left out: 753 tests, and 7 baseline tests, on 266 pages | 266 | 760 | 456 | 60.0 % |
| Baseline | the page gave text, without long repeats (one test per page) | – | 676 | 578 | 85.5 % |

Overall, as the benchmark averages its groups, 51.8 %; without the baseline group, 40.6 %. 672 pages were extracted and 13 failed. On one thread that is 375 pages a second, with a median of 2.0 ms a page.

Measured with Lamina PDF 0.2.0-alpha.3 (commit `40c4179`) on 8 November 2026, on Apple M1 Pro (8 cores, 16 GB of memory), macOS 27.0.1, with a load average of 8.4 when the run began.

## Where these numbers come from

One command runs every measurement on one machine before a release and writes the file this page reads, so no figure here is typed by hand. Each version of these docs keeps the numbers measured for it.

Each table also gives the machine's load average as its run began: how many jobs were running or waiting for a processor. A load average well above the core count means other jobs shared the machine, so the timings are slower than its best. For example, a load average of 12 on an 8-core machine means about four jobs were waiting for a processor at any moment.

Next: [make your first PDF](../../pages/start/first-pdf.md) and time it on your own machine.
