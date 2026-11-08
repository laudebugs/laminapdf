//! A report whose title, paragraphs and photo each start below wherever the one before ended.
//!
//! Each text box reports its `bottom`, and the next element starts a gap below it, so a summary
//! that wraps moves everything after it and empty notes take no space. The body continues on new
//! pages from the top margin. Run it with `cargo run -p laminapdf --example element_flow -- photo.jpg`.

use std::error::Error;

use laminapdf::{
    BuildOptions, Builder, Color, FontHandle, Image, Overflow, PageSize, Pt, RectPt, SaveOptions,
    Standard14::{Helvetica, HelveticaBold},
    StrokeStyle, Surface, TextBoxOptions, TextLayout,
};

const PAGE: PageSize = PageSize::A5;
const MARGIN: f64 = 48.0;
const WIDTH: f64 = PAGE.width.get() - 2.0 * MARGIN;
const BOTTOM: f64 = PAGE.height.get() - MARGIN;
const TITLE: Pt = Pt::new(18.0);
const BODY: Pt = Pt::new(11.0);
const GAP: f64 = 10.0;
/// The photo's height, for a 3:2 photo across the text column.
const PHOTO_HEIGHT: f64 = WIDTH / 1.5;

/// What the report says. Empty `notes` take no space; blank lines separate the body's paragraphs.
pub(crate) struct Report<'a> {
    pub(crate) title: &'a str,
    pub(crate) summary: &'a str,
    pub(crate) notes: &'a str,
    pub(crate) body: &'a str,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let photo = std::fs::read(args.next().ok_or("usage: element_flow <photo.jpg> [out-dir]")?)?;
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    let report = Report {
        title: "Third-quarter report",
        summary: "Revenue rose twelve per cent over the quarter, led by the new reader and its offline mode.",
        notes: "Figures are unaudited.",
        body: "Pages now appear as soon as they are read, and support requests about slow files fell by a third.",
    };
    let pdf = build(BuildOptions::new(), photo, &report)?;
    Ok(std::fs::write(out.join("element_flow.pdf"), pdf)?)
}

/// The report's bytes, with `photo` (a JPEG) below its notes. `options` can fix the file ID, the
/// producer and the creation date.
pub(crate) fn build(options: BuildOptions, photo: Vec<u8>, report: &Report) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options);
    let fonts = [b.standard_font(HelveticaBold)?, b.standard_font(Helvetica)?];
    let photo = Image::jpeg(photo)?;
    let mut page = b.page(PAGE);
    let mut y = header(page.surface(), &fonts, &photo, report)?;
    let (mut rest, mut new_page) = (report.body.trim_end(), false);
    loop {
        let s = page.surface();
        let layout = s.text_box(rest, &fonts[1], BODY, area(y), &TextBoxOptions::new())?;
        // A clipped box still drew its first lines: the rest goes on a new page, from its top.
        let Some(cut) = layout.overflow else { break };
        if cut == 0 && new_page {
            return Err("a line of the body is taller than the page".into());
        }
        (rest, y, new_page) = (rest[cut..].trim_start(), MARGIN, true);
        page.finish()?;
        page = b.page(PAGE);
    }
    page.finish()?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// The title with a rule under it, the summary, the notes and the photo, each below the one
/// before; returns where the body starts. Text that doesn't fit fails `text-overflow`.
fn header(s: &mut Surface, fonts: &[FontHandle; 2], photo: &Image, report: &Report) -> Result<f64, Box<dyn Error>> {
    let [bold, regular] = fonts;
    let must_fit = TextBoxOptions::new().with_overflow(Overflow::Error);
    let mut y = below(&s.text_box(report.title, bold, TITLE, area(MARGIN), &must_fit)?, MARGIN);
    // A decoration half a gap above the summary. It's stroked: a fill colour would colour the text too.
    let (rule, gray) = (RectPt::from_points(MARGIN, y - GAP / 2.0, WIDTH, 0.0), Color::gray(0.6));
    let line = StrokeStyle::default();
    s.stroke_color(gray).path(|p| _ = p.rect(rule)).stroke(&line);
    for text in [report.summary, report.notes] {
        y = below(&s.text_box(text, regular, BODY, area(y), &must_fit)?, y);
    }
    if y + PHOTO_HEIGHT > BOTTOM {
        return Err("the photo doesn't fit below the notes".into());
    }
    s.image(photo, RectPt::from_points(MARGIN, y, WIDTH, PHOTO_HEIGHT));
    Ok(y + PHOTO_HEIGHT + GAP)
}

/// Where the next element starts: a gap below `layout`'s last drawn line, or `y` if it drew none.
fn below(layout: &TextLayout, y: f64) -> f64 {
    layout.bottom().map_or(y, |bottom| bottom.get() + GAP)
}

/// The rest of the page's text column, from `y` down to the bottom margin.
fn area(y: f64) -> RectPt {
    RectPt::from_points(MARGIN, y, WIDTH, (BOTTOM - y).max(0.0))
}
