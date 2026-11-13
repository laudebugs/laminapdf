---
title: "Rust API (rustdoc)"
description: "Where to read the API documentation of each Lamina PDF crate, and what each crate does."
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/rust/"
---

# Rust API (rustdoc)

Where to read the API documentation of each Lamina PDF crate, and what each crate does.

Every Lamina PDF crate on crates.io has its API documentation on docs.rs. Most programs depend on one crate, [`laminapdf`](https://docs.rs/laminapdf/0.2.0/laminapdf/), so start there. It holds `Document`, `Builder`, `Error`, `Warning`, the jobs and `CancelToken`, and it wraps the other crates.

## The other crates

You need these only to reach something `laminapdf` doesn't wrap, or to read how a part works:

- [`laminapdf-syntax`](https://docs.rs/laminapdf-syntax/0.2.0/laminapdf_syntax/) reads and writes PDF objects, and holds the error codes.
- [`laminapdf-cos`](https://docs.rs/laminapdf-cos/0.2.0/laminapdf_cos/) opens a file, repairs a broken one, and keeps your edits apart from the bytes you opened.
- [`laminapdf-model`](https://docs.rs/laminapdf-model/0.2.0/laminapdf_model/) gives typed views: pages, the outline, metadata and the structure tree.
- [`laminapdf-ops`](https://docs.rs/laminapdf-ops/0.2.0/laminapdf_ops/) holds the edits, their transactions, and undo and redo.
- [`laminapdf-save`](https://docs.rs/laminapdf-save/0.2.0/laminapdf_save/) chooses how to save and writes the file.
- [`laminapdf-content`](https://docs.rs/laminapdf-content/0.2.0/laminapdf_content/) reads a page's drawing instructions.
- [`laminapdf-text`](https://docs.rs/laminapdf-text/0.2.0/laminapdf_text/) loads, shapes and embeds fonts, lays out text and reads text back.
- [`laminapdf-build`](https://docs.rs/laminapdf-build/0.2.0/laminapdf_build/) builds new pages and documents.
- [`laminapdf-extract`](https://docs.rs/laminapdf-extract/0.2.0/laminapdf_extract/) turns pages into blocks, tables, Markdown and chunks.
- [`laminapdf-source`](https://docs.rs/laminapdf-source/0.2.0/laminapdf_source/) reads files from disk or in ranges, as they download.
- [`laminapdf-render`](https://docs.rs/laminapdf-render/0.2.0/laminapdf_render/) defines how a renderer draws pages into pixels.
- [`laminapdf-render-pdfium`](https://docs.rs/laminapdf-render-pdfium/0.2.0/laminapdf_render_pdfium/) draws pages with PDFium.

[Install](../../pages/install.md#every-published-package) lists every crate with its page on crates.io.

## Build the docs yourself

To read the docs of a change that isn't released yet, build them from a checkout of Lamina PDF. This command builds every crate's docs and opens them in your browser:

```sh
cargo doc --workspace --no-deps --all-features --open
```

For how the packages version together and what an upgrade can change, read the [stability policy](../../pages/reference/stability.md) next.
