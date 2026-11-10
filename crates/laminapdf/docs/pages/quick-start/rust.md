---
title: "Rust"
description: "Generate a PDF with the Lamina PDF crate."
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/quick-start/rust/"
---

# Rust

Generate a PDF with the Lamina PDF crate.

The `laminapdf` crate is the one to depend on: it wraps Lamina PDF's core crates in one API. Add it with `cargo add laminapdf@=0.2.0-alpha.5`, then build a document. [Install](../../pages/install.md#rust-laminapdf) explains the version. This is the `hello` example, which is tested on every change:

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

To run it, save it as `src/main.rs` in a project that depends on `laminapdf`, then run `cargo run`. It writes `target/examples-out/hello.pdf`, or `hello.pdf` in the folder you pass: `cargo run -- out`.

- **Same input, same bytes:** `build` takes `BuildOptions`: fix the file ID, the producer and the creation date in them, and the output never changes; see [how Lamina PDF thinks about a PDF](../../pages/concepts/how-lamina-thinks.md#same-input-same-bytes).
- **Stopping a save:** `save` runs to the end. To let another thread stop it, pass a `CancelToken` to `save_with`, or call `save_job` for a job that you step through with a work budget.
- The [Rust API reference](../../pages/reference/rust.md) documents every public item.
