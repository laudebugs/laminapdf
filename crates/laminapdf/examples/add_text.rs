//! Type on a page as page content: a note in Helvetica with one bold word, and a greeting in two
//! scripts set in fonts given by their bytes, the second font drawing the Arabic the first lacks.
//!
//! The text becomes part of the page: it prints, every reader draws it, and it copies and
//! searches as typed. The boxes are in page space (points from the page's top-left), as a viewer
//! reports where a pointer went. In this tagged report each text is a paragraph that screen
//! readers read. Fonts given by their bytes need the WebAssembly text module; Helvetica, Times
//! and Courier need no bytes.
//!
//! ```sh
//! cargo run -p laminapdf --example add_text -- in.pdf latin.ttf arabic.ttf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotFont, ContentId, Document, Op, OpenOptions, PdfDate, Pt, RectPt, SaveMode, SaveOptions, StandardFamily,
    TextSpan,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: add_text in.pdf latin.ttf arabic.ttf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let latin = std::fs::read(args.next().ok_or(usage)?)?;
    let arabic = std::fs::read(args.next().ok_or(usage)?)?;
    // When the text was typed, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let typed = add_text(std::fs::read(input)?, latin, arabic, at)?;
    std::fs::write(out.join("add_text.pdf"), typed)?;
    Ok(())
}

// Where the texts go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// The note: `[x0, y0, x1, y1]`.
const NOTE: [f64; 4] = [56.0, 690.0, 300.0, 712.0];
/// The greeting, under the note.
const GREETING: [f64; 4] = [56.0, 718.0, 300.0, 746.0];
/// The size of the greeting, in points.
const GREETING_SIZE: f64 = 16.0;

/// The document with a note and a greeting typed on its first page, saved incrementally.
pub(crate) fn add_text(
    bytes: Vec<u8>,
    latin: Vec<u8>,
    arabic: Vec<u8>,
    at: PdfDate,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let id = |name: &str| ContentId::new(name).ok_or("an ID is ASCII letters, digits, '-', '_' or '.'");

    let helvetica = AnnotFont::Standard(StandardFamily::Helvetica);
    let runs = vec![TextSpan::plain("Reviewed "), TextSpan::plain("twice").with_bold(true)];
    let note = doc
        .text_at(0, id("note")?, area(NOTE), runs, at)?
        .with_fonts(vec![helvetica], 12.0);

    // The first font draws the Latin, the second what the first lacks.
    let fonts = vec![
        AnnotFont::Font(doc.add_blob(latin)),
        AnnotFont::Font(doc.add_blob(arabic)),
    ];
    let hello = vec![TextSpan::plain("Hello \u{645}\u{631}\u{62d}\u{628}\u{627}")];
    let greeting = doc
        .text_at(0, id("greeting")?, area(GREETING), hello, at)?
        .with_fonts(fonts, GREETING_SIZE);

    doc.apply(vec![Op::from(note), Op::from(greeting)])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// A box in page space from its corners.
fn area([x0, y0, x1, y1]: [f64; 4]) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}
