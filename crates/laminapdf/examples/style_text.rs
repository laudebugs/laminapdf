//! Change how text a page already shows looks: its weight, colour and underline.
//!
//! Finds the first run that says "world" and makes just that word bold, red and underlined,
//! leaving the rest of the run as it was. A style names runs, or glyphs inside one, and changes
//! only the fields it sets: a font family, a size, a colour, bold, italic, underline, raised or
//! lowered text, and the alignment and spacing of the run's lines. The text itself doesn't change,
//! so it still copies and searches as before. Bold and italic pick the face of a standard font
//! such as Helvetica; for another font, name the bold face as the font.
//!
//! ```sh
//! cargo run -p laminapdf --example style_text -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{AnnotColor, Document, OpenOptions, SaveOptions, SetTextStyle, TextRunRef};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: style_text in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let styled = style(std::fs::read(input)?)?;
    std::fs::write(out.join("style_text.pdf"), styled)?;
    Ok(())
}

/// The document with "world" made bold, red and underlined on its first page.
pub(crate) fn style(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let runs = doc.text_runs(0)?;
    let found = runs
        .iter()
        .find(|r| r.read_only.is_none() && r.text.contains("world"))
        .ok_or("no editable run says \"world\"")?;
    // The run's glyphs are numbered from `run.start`; the word is five of them.
    let at = u32::try_from(found.text.find("world").ok_or("no \"world\"")?)?;
    let word = TextRunRef::new(found.run.object, found.run.start + at, found.run.start + at + 5);
    let mut style = SetTextStyle::new(doc.page_info(0)?.id, vec![word]);
    style.bold = Some(true);
    style.color = Some(AnnotColor::Rgb(0.8, 0.0, 0.0));
    style.underline = Some(true);
    doc.apply(vec![style.into()])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
