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
    std::fs::write(out.join("hello.pdf"), build()?)?;
    Ok(())
}

/// The document's bytes: the same on every run and every platform.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    // Everything that would come from the environment (file ID, dates) is given.
    let options = BuildOptions::default()
        .with_id(*b"lamina-ex-hello!")
        .with_producer("lamina examples")
        .with_creation_date("D:20261030120000Z".parse()?);
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
