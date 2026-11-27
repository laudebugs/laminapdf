//! Put the same marks on every page of a document: a hideable CONFIDENTIAL watermark, a header
//! with the file's name and the date, a footer that numbers the pages, and a Bates number in the
//! corner of each page.
//!
//! The marks become part of each page: they print and every reader draws them. The watermark sits
//! in an optional content group, so a reader's layers panel can switch it off, and in a tagged
//! file like this report every mark is an artifact that screen readers skip. A header's `{page}`
//! and `{pages}` are filled when the edit is applied; the Bates numbers count from where you say
//! (number a second file from where the first ended). The marks stay changeable after a save:
//! [`Document::page_marks`] lists them by the IDs below.
//!
//! ```sh
//! cargo run -p laminapdf --example add_page_marks -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotColor, AnnotFont, ContentId, Document, MarkContent, Op, Opacity, OpenOptions, PdfDate, SaveMode, SaveOptions,
    Slot, StandardFamily,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: add_page_marks in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the marks were made, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let marked = add_page_marks(std::fs::read(&input)?, "report.pdf", "27 Nov 2026", at)?;
    std::fs::write(out.join("add_page_marks.pdf"), marked)?;
    Ok(())
}

/// The document with its pages marked, saved incrementally. `file` and `date` are what the
/// header's `{file}` and `{date}` stand for, as you want them written.
pub(crate) fn add_page_marks(bytes: Vec<u8>, file: &str, date: &str, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let id = |name: &str| ContentId::new(name).ok_or("an ID is ASCII letters, digits, '-', '_' or '.'");
    // Every page, counted from 0.
    let pages: Vec<u32> = (0..doc.page_count()?).collect();

    // A red CONFIDENTIAL across each page, turned 45 degrees and faint, which a reader can hide.
    let text = MarkContent::Text {
        text: "CONFIDENTIAL".into(),
        fonts: vec![AnnotFont::Standard(StandardFamily::Helvetica)],
        size: 90.0,
        color: Some(AnnotColor::Rgb(0.8, 0.1, 0.1)),
    };
    let watermark = doc
        .watermark(&pages, id("confidential")?, text, at)?
        .with_opacity(Opacity::new(0.2).ok_or("an opacity is from 0 to 1")?)
        .with_angle(45.0)
        .hideable();

    // The file's name and the date at the top right, and the page's number at the bottom centre,
    // clear of this report's own header at the top left and page number at the bottom right: slots
    // in the order top left, centre, right, then bottom left, centre, right.
    let mut slots: [Option<String>; 6] = Default::default();
    slots[2] = Some("{file} \u{b7} {date}".into());
    slots[4] = Some("Page {page} of {pages}".into());
    let folio = doc
        .header_footer(&pages, id("folio")?, slots, at)?
        .with_fonts(vec![AnnotFont::Standard(StandardFamily::Helvetica)], 9.0)
        .with_color(AnnotColor::Gray(0.35))
        .with_fields(date, file);

    // ACME-000101, ACME-000102 and on, at the bottom left.
    let numbers = doc
        .bates(&pages, id("bates")?, 101, Slot::BottomLeft, at)?
        .with_text("ACME-", 6, "")
        .with_fonts(vec![AnnotFont::Standard(StandardFamily::Courier)], 9.0);

    doc.apply(vec![Op::from(watermark), Op::from(folio), Op::from(numbers)])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}
