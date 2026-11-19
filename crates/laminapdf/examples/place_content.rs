//! Brand a report: a pale band behind its title, the company's logo over the band's right end,
//! and its wordmark, a page of another PDF kept as vectors and text, at the foot of the page.
//!
//! Each is placed as page content, not as a comment: it prints and every reader draws it. The
//! boxes are in page space (points from the page's top-left), as a viewer reports where a
//! pointer went. In this tagged report the logo and the wordmark are figures with alternate
//! text, and the band is decoration, which screen readers skip.
//!
//! ```sh
//! cargo run -p laminapdf --example place_content -- in.pdf logo.png wordmark.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Alt, AnnotColor, ContentId, Document, Layer, Op, OpenOptions, PathSpec, PdfDate, Pt, RectPt, SaveMode, SaveOptions,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: place_content in.pdf logo.png wordmark.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let logo = std::fs::read(args.next().ok_or(usage)?)?;
    let wordmark = std::fs::read(args.next().ok_or(usage)?)?;
    // When the report was branded, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let branded = place_content(std::fs::read(input)?, logo, wordmark, at)?;
    std::fs::write(out.join("place_content.pdf"), branded)?;
    Ok(())
}

// Where each goes on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// The band behind the title: `[x0, y0, x1, y1]`.
const BAND: [f64; 4] = [44.0, 68.0, 551.0, 110.0];
/// The logo, over the band's right end, as wide again as it is tall, as the image is.
const LOGO: [f64; 4] = [455.0, 65.0, 551.0, 113.0];
/// The wordmark, at the foot of the page, the size of its page.
const WORDMARK: [f64; 4] = [56.0, 740.0, 176.0, 800.0];

/// The document branded: a band, a logo and a wordmark on its first page, saved incrementally.
pub(crate) fn place_content(
    bytes: Vec<u8>,
    logo: Vec<u8>,
    wordmark: Vec<u8>,
    at: PdfDate,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let id = |name: &str| ContentId::new(name).ok_or("an ID is ASCII letters, digits, '-', '_' or '.'");

    // Behind the page's own content, so the title stays on top of it.
    let band = PathSpec::Rect {
        rect: area(BAND),
        radius: 6.0,
    };
    let pale_blue = AnnotColor::Rgb(0.89, 0.93, 0.98);
    let band = doc
        .shape_on(0, id("title-band")?, band, Some(pale_blue), None, at)?
        .with_layer(Layer::Behind)
        .with_alt(Alt::Decorative);

    let logo = doc
        .image_at(0, id("logo")?, area(LOGO), logo, at)?
        .with_alt(Alt::Text("Northwind Traders".into()));

    let wordmark = doc
        .page_at(0, id("wordmark")?, area(WORDMARK), wordmark, 0, at)?
        .with_alt(Alt::Text("Northwind, the company's wordmark".into()));

    doc.apply(vec![Op::from(band), Op::from(logo), Op::from(wordmark)])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// A box in page space from its corners.
fn area([x0, y0, x1, y1]: [f64; 4]) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}
