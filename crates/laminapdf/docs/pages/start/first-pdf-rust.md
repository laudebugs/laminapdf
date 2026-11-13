---
title: "Your first PDF in Rust"
description: "Write hello.pdf from a Rust program in five minutes."
version: "0.2.0"
url: "https://docs.laminapdf.com/start/first-pdf-rust/"
---

# Your first PDF in Rust

Write hello.pdf from a Rust program in five minutes.

You'll write `hello.pdf`, one A4 page that says "Hello, world!", from a small Rust program. It takes about five minutes, most of it for the first build.

## Make a project

You need Rust 1.97 or newer. Create a project and add the `laminapdf` crate:

```sh
cargo new hello-pdf && cd hello-pdf
cargo add laminapdf@0.2.0
```

This adds the version these docs describe. Cargo reads it as this release or a later one in the same `0.x` line, which the [stability policy](../../pages/reference/stability.md) promises needs no change to your code. [Install](../../pages/install.md#rust-laminapdf) covers the crate's features.

## Write the page

Replace `src/main.rs` with this program. It builds the page and writes it to `hello.pdf`:

**Rust**

`crates/laminapdf/examples/hello.rs`:

````rust
//! The smallest document: one A4 page with a line of text in a standard 14 font.
//!
//! ```sh
//! cargo run -p laminapdf --example hello -- [out-dir]   # writes <out-dir>/hello.pdf (default target/examples-out)
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{BuildOptions, Builder, PageSize, PointPt, Pt, SaveOptions, Standard14};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("hello.pdf"), build(BuildOptions::new())?)?;
    Ok(())
}

/// The document's bytes. `options` can fix the file ID, the producer and the creation date, so
/// that the same input gives the same bytes on every run.
pub(crate) fn build(options: BuildOptions) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options);
    let helvetica = b.standard_font(Standard14::Helvetica)?;
    let mut page = b.page(PageSize::A4);
    // Page space: points from the top-left corner, y down; `y` is the text's baseline.
    page.surface().text(
        "Hello, world!",
        &helvetica,
        Pt::new(24.0),
        PointPt::from_points(72.0, 96.0),
    )?;
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [hello.pdf](https://docs.laminapdf.com/examples/pdf/hello.pdf)

## Run it

```sh
cargo run
```

The comment at the top of the file shows how Lamina PDF's own repository runs it; in your project, `cargo run` is all it takes. The program writes `target/examples-out/hello.pdf`; `cargo run -- out` writes `out/hello.pdf` instead. Open it in any PDF viewer: one page, with "Hello, world!" near its top-left corner.

## What happened

`Builder` made an A4 page, and the page's surface drew the text in Helvetica, one of the [standard 14 fonts](../../pages/reference/glossary.md) every PDF viewer has, so nothing was embedded. Points count from the page's top-left corner, and the text's `y` is its baseline. `finish` turned the builder into a document, and `save` turned the document into bytes.

`build` takes `BuildOptions`, which can fix the file ID, the producer and the creation date, so the same program writes the same bytes on every run. The examples on this site use that to check their output byte for byte.

Next, see [how Lamina PDF thinks about a PDF](../../pages/concepts/how-lamina-thinks.md) and [where things go on a page](../../pages/concepts/pages-and-coordinates.md). For your own fonts and text that wraps, read [text and fonts](../../pages/guides/fonts-and-text.md).
